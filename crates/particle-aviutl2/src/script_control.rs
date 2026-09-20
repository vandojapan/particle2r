//! Legacy Script Control bridge.
//!
//! AviUtl2 does not expose another effect's Lua state to a native filter.  We
//! therefore read the source text from the standard 「スクリプト制御」 effect
//! and evaluate the documented `xyz`, `xyzd`, and `vector` functions in a
//! private Lua 5.1 state supplied by AviUtl2's own `lua.dll`.

use particle_core::ScriptMotion;
use std::{
    collections::HashMap,
    ffi::{CString, c_char, c_void},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    ptr,
    sync::{Mutex, OnceLock},
    time::SystemTime,
};

const LUA_GLOBALSINDEX: i32 = -10002;
const LUA_TFUNCTION: i32 = 6;
const MAX_CURVE_SAMPLES: usize = 8192;
const SAMPLE_RATE: f64 = 240.0;

type LuaState = *mut c_void;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn LoadLibraryExW(name: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    fn MultiByteToWideChar(
        code_page: u32,
        flags: u32,
        source: *const u8,
        source_length: i32,
        destination: *mut u16,
        destination_length: i32,
    ) -> i32;
}

const MAX_SCRIPT_BYTES: u64 = 1_048_576;

#[derive(Clone)]
struct ScriptFileEntry {
    length: u64,
    modified: Option<SystemTime>,
    source: String,
}

static SCRIPT_FILES: OnceLock<Mutex<HashMap<PathBuf, ScriptFileEntry>>> = OnceLock::new();

/// Values exposed through the legacy `obj.getvalue("N.x", t)` API.
/// `legacy_layer` is one-based, matching the AviUtl 1.x script convention.
#[derive(Clone, Debug)]
pub(super) struct HostValueTrace {
    legacy_layer: u32,
    step: f64,
    values: Vec<[f64; 3]>,
}

impl HostValueTrace {
    pub(super) fn new(legacy_layer: u32, step: f64, values: Vec<[f64; 3]>) -> Self {
        Self {
            legacy_layer,
            step,
            values,
        }
    }

    fn lua_prelude(&self) -> String {
        let mut source = format!(
            "local __particle2r_layer={};local __particle2r_step={:.17};local __particle2r_values={{",
            self.legacy_layer, self.step
        );
        for value in &self.values {
            let _ = write!(
                source,
                "{{{:.17},{:.17},{:.17}}},",
                value[0], value[1], value[2]
            );
        }
        source.push_str(
            r#"}
function obj.getvalue(key,t)
  if type(key)~="string" or type(t)~="number" then return 0 end
  local layer,axis=string.match(key,"^(%d+)%.([xyz])$")
  if not layer then layer,axis=string.match(key,"^layer(%d+)%.([xyz])$") end
  if tonumber(layer)~=__particle2r_layer then return 0 end
  local axis_index=axis=="x" and 1 or axis=="y" and 2 or axis=="z" and 3 or nil
  local count=#__particle2r_values
  if not axis_index or count==0 then return 0 end
  if count==1 or t<=0 then return __particle2r_values[1][axis_index] end
  local position=t/__particle2r_step
  if position>=count-1 then return __particle2r_values[count][axis_index] end
  local lower=math.floor(position)+1
  local fraction=position-math.floor(position)
  local a=__particle2r_values[lower][axis_index]
  local b=__particle2r_values[lower+1][axis_index]
  return a+(b-a)*fraction
end
"#,
        );
        source
    }
}

struct LuaApi {
    new_state: unsafe extern "C" fn() -> LuaState,
    open_libs: unsafe extern "C" fn(LuaState),
    load_buffer: unsafe extern "C" fn(LuaState, *const c_char, usize, *const c_char) -> i32,
    pcall: unsafe extern "C" fn(LuaState, i32, i32, i32) -> i32,
    get_field: unsafe extern "C" fn(LuaState, i32, *const c_char),
    lua_type: unsafe extern "C" fn(LuaState, i32) -> i32,
    push_number: unsafe extern "C" fn(LuaState, f64),
    to_number: unsafe extern "C" fn(LuaState, i32) -> f64,
    to_string: unsafe extern "C" fn(LuaState, i32, *mut usize) -> *const c_char,
    get_top: unsafe extern "C" fn(LuaState) -> i32,
    set_top: unsafe extern "C" fn(LuaState, i32),
    close: unsafe extern "C" fn(LuaState),
}

impl LuaApi {
    fn load() -> Result<Self, String> {
        let module_name = wide("lua.dll");
        // AviUtl2 normally has already loaded lua.dll. The development path
        // fallback also lets the bridge be exercised by local tests/tools.
        let mut module = unsafe { GetModuleHandleW(module_name.as_ptr()) };
        if module.is_null() {
            module = unsafe { LoadLibraryW(module_name.as_ptr()) };
        }
        if module.is_null() {
            let relative = PathBuf::from(".aviutl2-cli/development/lua.dll");
            let fallback = std::env::current_dir()
                .map(|directory| directory.join(&relative))
                .ok()
                .filter(|path| path.is_file())
                .unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../..")
                        .join(relative)
                });
            if fallback.is_file() {
                module = unsafe {
                    LoadLibraryExW(
                        wide(&fallback.to_string_lossy()).as_ptr(),
                        ptr::null_mut(),
                        0x0000_0008, // LOAD_WITH_ALTERED_SEARCH_PATH
                    )
                };
            }
        }
        if module.is_null() {
            return Err(format!(
                "AviUtl2 の lua.dll を読み込めません ({})",
                std::io::Error::last_os_error()
            ));
        }

        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                let pointer = unsafe { GetProcAddress(module, concat!($name, "\0").as_ptr()) };
                if pointer.is_null() {
                    return Err(format!("lua.dll に {} がありません", $name));
                }
                unsafe { std::mem::transmute::<*mut c_void, $ty>(pointer) }
            }};
        }
        Ok(Self {
            new_state: symbol!("luaL_newstate", unsafe extern "C" fn() -> LuaState),
            open_libs: symbol!("luaL_openlibs", unsafe extern "C" fn(LuaState)),
            load_buffer: symbol!(
                "luaL_loadbuffer",
                unsafe extern "C" fn(LuaState, *const c_char, usize, *const c_char) -> i32
            ),
            pcall: symbol!(
                "lua_pcall",
                unsafe extern "C" fn(LuaState, i32, i32, i32) -> i32
            ),
            get_field: symbol!(
                "lua_getfield",
                unsafe extern "C" fn(LuaState, i32, *const c_char)
            ),
            lua_type: symbol!("lua_type", unsafe extern "C" fn(LuaState, i32) -> i32),
            push_number: symbol!("lua_pushnumber", unsafe extern "C" fn(LuaState, f64)),
            to_number: symbol!("lua_tonumber", unsafe extern "C" fn(LuaState, i32) -> f64),
            to_string: symbol!(
                "lua_tolstring",
                unsafe extern "C" fn(LuaState, i32, *mut usize) -> *const c_char
            ),
            get_top: symbol!("lua_gettop", unsafe extern "C" fn(LuaState) -> i32),
            set_top: symbol!("lua_settop", unsafe extern "C" fn(LuaState, i32)),
            close: symbol!("lua_close", unsafe extern "C" fn(LuaState)),
        })
    }
}

struct LuaVm {
    api: LuaApi,
    state: LuaState,
}

impl LuaVm {
    fn new(
        source: &str,
        layer: u32,
        object_time: f64,
        object_total: f64,
        frame: u32,
        frame_total: u32,
        frame_rate: f64,
        host_values: Option<&HostValueTrace>,
    ) -> Result<Self, String> {
        if source.len() > 1_048_576 {
            return Err("スクリプト制御コードが1MiBを超えています".to_string());
        }
        let api = LuaApi::load()?;
        let state = unsafe { (api.new_state)() };
        if state.is_null() {
            return Err("Lua状態を作成できません".to_string());
        }
        unsafe { (api.open_libs)(state) };
        let mut vm = Self { api, state };
        // AviUtl2 exposes zero-based layers while the original Lua API uses
        // one-based layer numbers.
        let mut prelude = format!(
            "os=nil;io=nil;package=nil;debug=nil;coroutine=nil;obj={{layer={},time={:.17},totaltime={:.17},frame={},totalframe={},framerate={:.17}}}\n",
            layer.saturating_add(1),
            object_time,
            object_total,
            frame,
            frame_total,
            frame_rate
        );
        if let Some(values) = host_values {
            prelude.push_str(&values.lua_prelude());
        } else {
            prelude.push_str("function obj.getvalue(key,t) return 0 end\n");
        }
        prelude.push_str(source);
        vm.exec(&prelude)?;
        Ok(vm)
    }

    fn exec(&mut self, source: &str) -> Result<(), String> {
        let name = b"particle2r-script-control\0";
        let status = unsafe {
            (self.api.load_buffer)(
                self.state,
                source.as_ptr().cast(),
                source.len(),
                name.as_ptr().cast(),
            )
        };
        if status != 0 {
            return Err(self.take_error("Luaコンパイルエラー"));
        }
        let status = unsafe { (self.api.pcall)(self.state, 0, 0, 0) };
        if status != 0 {
            return Err(self.take_error("Lua実行エラー"));
        }
        Ok(())
    }

    fn has_function(&self, name: &str) -> bool {
        let Ok(name) = CString::new(name) else {
            return false;
        };
        let top = unsafe { (self.api.get_top)(self.state) };
        unsafe { (self.api.get_field)(self.state, LUA_GLOBALSINDEX, name.as_ptr()) };
        let found = unsafe { (self.api.lua_type)(self.state, -1) == LUA_TFUNCTION };
        unsafe { (self.api.set_top)(self.state, top) };
        found
    }

    fn call(&mut self, name: &str, time: f64, count: usize) -> Result<Vec<f64>, String> {
        let name = CString::new(name).map_err(|_| "不正な関数名です".to_string())?;
        let top = unsafe { (self.api.get_top)(self.state) };
        unsafe {
            (self.api.get_field)(self.state, LUA_GLOBALSINDEX, name.as_ptr());
            (self.api.push_number)(self.state, time);
        }
        let status = unsafe { (self.api.pcall)(self.state, 1, count as i32, 0) };
        if status != 0 {
            return Err(self.take_error("Lua関数エラー"));
        }
        let mut values = Vec::with_capacity(count);
        for index in 0..count {
            let stack_index = -(count as i32) + index as i32;
            let value = unsafe { (self.api.to_number)(self.state, stack_index) };
            if !value.is_finite() {
                unsafe { (self.api.set_top)(self.state, top) };
                return Err(format!(
                    "{} の戻り値が有限数ではありません",
                    name.to_string_lossy()
                ));
            }
            values.push(value);
        }
        unsafe { (self.api.set_top)(self.state, top) };
        Ok(values)
    }

    fn take_error(&mut self, prefix: &str) -> String {
        let mut length = 0usize;
        let pointer = unsafe { (self.api.to_string)(self.state, -1, &mut length) };
        let detail = if pointer.is_null() {
            String::new()
        } else {
            let bytes = unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), length) };
            String::from_utf8_lossy(bytes).into_owned()
        };
        unsafe { (self.api.set_top)(self.state, 0) };
        if detail.is_empty() {
            prefix.to_string()
        } else {
            format!("{prefix}: {detail}")
        }
    }
}

impl Drop for LuaVm {
    fn drop(&mut self) {
        if !self.state.is_null() {
            unsafe { (self.api.close)(self.state) };
            self.state = ptr::null_mut();
        }
    }
}

pub(super) fn build_motion(
    output_source: Option<&str>,
    behavior_source: Option<&str>,
    object_time: f64,
    object_total: f64,
    frame: u32,
    frame_total: u32,
    frame_rate: f64,
    lifetime: f64,
    layer: u32,
    host_values: Option<&HostValueTrace>,
) -> Result<ScriptMotion, String> {
    if output_source.is_none() && behavior_source.is_none() {
        return Ok(ScriptMotion::default());
    }
    let mut motion = ScriptMotion::default();

    if let Some(source) = output_source.filter(|source| !source.trim().is_empty()) {
        let mut vm = LuaVm::new(
            source,
            layer,
            object_time,
            object_total,
            frame,
            frame_total,
            frame_rate,
            host_values,
        )?;
        let (name, result_count, has_direction) = if vm.has_function("xyzd") {
            ("xyzd", 5, true)
        } else if vm.has_function("xyz") {
            ("xyz", 3, false)
        } else {
            return Err("@出力タイプ8用の xyz(t) または xyzd(t) がありません".to_string());
        };
        let (step, count) = curve_layout(object_time.max(0.0));
        motion.output_step = step;
        motion.output_has_direction = has_direction;
        motion.output.reserve(count);
        for index in 0..count {
            let values = vm.call(name, index as f64 * step, result_count)?;
            let mut point = [0.0; 5];
            point[..result_count].copy_from_slice(&values);
            motion.output.push(point);
        }
    }

    if let Some(source) = behavior_source.filter(|source| !source.trim().is_empty()) {
        let mut vm = LuaVm::new(
            source,
            layer,
            object_time,
            object_total,
            frame,
            frame_total,
            frame_rate,
            host_values,
        )?;
        if !vm.has_function("vector") {
            return Err("@挙動用の vector(t) がありません".to_string());
        }
        let (step, count) = curve_layout(lifetime.max(0.0));
        motion.behavior_step = step;
        motion.behavior_position.reserve(count);
        motion.behavior_position.push([0.0; 3]);
        for index in 1..count {
            let midpoint = (index as f64 - 0.5) * step;
            let velocity = vm.call("vector", midpoint, 3)?;
            let previous = motion.behavior_position[index - 1];
            motion.behavior_position.push([
                previous[0] + velocity[0] * step,
                previous[1] + velocity[1] * step,
                previous[2] + velocity[2] * step,
            ]);
        }
    }
    Ok(motion)
}

pub(super) fn load_source_file(path: &Path) -> Result<String, String> {
    let metadata =
        fs::metadata(path).map_err(|error| format!("{} を開けません: {error}", path.display()))?;
    if metadata.len() > MAX_SCRIPT_BYTES {
        return Err(format!("{} が1MiBを超えています", path.display()));
    }
    let modified = metadata.modified().ok();
    let cache = SCRIPT_FILES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(entries) = cache.lock()
        && let Some(entry) = entries.get(path)
        && entry.length == metadata.len()
        && entry.modified == modified
    {
        return Ok(entry.source.clone());
    }

    let bytes =
        fs::read(path).map_err(|error| format!("{} を読めません: {error}", path.display()))?;
    let source = decode_script_bytes(&bytes)?;
    if let Ok(mut entries) = cache.lock() {
        entries.insert(
            path.to_path_buf(),
            ScriptFileEntry {
                length: metadata.len(),
                modified,
                source: source.clone(),
            },
        );
    }
    Ok(source)
}

fn decode_script_bytes(bytes: &[u8]) -> Result<String, String> {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    if let Ok(source) = std::str::from_utf8(bytes) {
        return Ok(source.to_string());
    }
    if bytes.len() > i32::MAX as usize {
        return Err("スクリプトファイルが大きすぎます".to_string());
    }
    // The bundled ver3.54B files and user scripts from AviUtl 1.x commonly use CP932.
    let required = unsafe {
        MultiByteToWideChar(
            932,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            ptr::null_mut(),
            0,
        )
    };
    if required <= 0 {
        return Err(format!(
            "UTF-8/Shift_JISとして読めません ({})",
            std::io::Error::last_os_error()
        ));
    }
    let mut wide = vec![0u16; required as usize];
    let written = unsafe {
        MultiByteToWideChar(
            932,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            wide.as_mut_ptr(),
            required,
        )
    };
    if written <= 0 {
        return Err(format!(
            "Shift_JISを変換できません ({})",
            std::io::Error::last_os_error()
        ));
    }
    Ok(String::from_utf16_lossy(&wide[..written as usize]))
}

pub(super) fn curve_layout(duration: f64) -> (f64, usize) {
    let requested = (duration * SAMPLE_RATE).ceil() as usize + 1;
    let count = requested.clamp(2, MAX_CURVE_SAMPLES);
    let step = if duration > 0.0 {
        duration / (count - 1) as f64
    } else {
        1.0 / SAMPLE_RATE
    };
    (step, count)
}

pub(super) fn decode_effect_text(value: &str) -> String {
    let trimmed = value.trim();
    let content = if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('t') => result.push('\t'),
            Some('"') => result.push('"'),
            Some('\\') => result.push('\\'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }
    result
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_effect_text_is_decoded() {
        assert_eq!(
            decode_effect_text("\"function xyz(t)\\nreturn t,0,0\\nend\""),
            "function xyz(t)\nreturn t,0,0\nend"
        );
        assert_eq!(
            decode_effect_text("function vector(t)\\n\\treturn 1,2,3\\nend"),
            "function vector(t)\n\treturn 1,2,3\nend"
        );
    }

    #[test]
    fn curve_layout_is_bounded() {
        assert_eq!(curve_layout(0.0).1, 2);
        assert_eq!(curve_layout(10_000.0).1, MAX_CURVE_SAMPLES);
    }

    #[test]
    fn documented_lua_functions_build_motion_curves() {
        let source = r#"
            function xyzd(t)
                return 10*t, math.sin(math.pi*t), 3, math.pi/2, 0
            end
            function vector(t)
                return 1, 2, 3
            end
        "#;
        let motion = build_motion(
            Some(source),
            Some(source),
            1.0,
            2.0,
            30,
            60,
            30.0,
            1.0,
            1,
            None,
        )
        .unwrap();
        let output = motion.output.last().unwrap();
        assert!((output[0] - 10.0).abs() < 1e-9);
        assert!((output[3] - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        let behavior = motion.behavior_position.last().unwrap();
        assert!((behavior[0] - 1.0).abs() < 1e-9);
        assert!((behavior[1] - 2.0).abs() < 1e-9);
        assert!((behavior[2] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn documented_xyzd_moves_inward_and_degvxyz_rotates_triangle() {
        let source = r#"
            function xyzd(t)
                local r=300
                local degxy=math.pi*t*2
                local degz=0
                local x=r*math.sin(degxy)
                local y=r*math.cos(degxy)
                local z=0
                degxy=degxy-math.pi
                return x,y,z,degxy,degz
            end
        "#;
        let motion =
            build_motion(Some(source), None, 3.0, 3.0, 90, 90, 30.0, 3.0, 1, None).unwrap();
        assert!(motion.output_has_direction);

        // A triangle makes Z rotation visible. Particle zero starts at
        // (0, 300), travels 100 px/s toward the centre, and spins 60 deg/s.
        let config = particle_core::ParticleConfig {
            frequency: 1.0,
            speed: 100.0,
            direction_degrees: 0.0,
            spread_degrees: 0.0,
            initial_rotation_z_degrees: 0.0,
            rotation_z_degrees_per_second: 60.0,
            lifetime: 3.0,
            script_motion: motion,
            ..Default::default()
        };
        let after_one_second = particle_core::sample(&config, 1.0)
            .into_iter()
            .find(|particle| particle.id == 0)
            .unwrap();
        assert!(after_one_second.x.abs() < 0.001);
        assert!((after_one_second.y - 200.0).abs() < 0.01);
        assert!((after_one_second.rz - 60.0).abs() < 0.001);

        let before_end = particle_core::sample(&config, 2.999)
            .into_iter()
            .find(|particle| particle.id == 0)
            .unwrap();
        assert!(before_end.x.abs() < 0.001);
        assert!(before_end.y.abs() < 0.2);
        assert!(
            particle_core::sample(&config, 3.0)
                .into_iter()
                .all(|particle| particle.id != 0)
        );
    }

    #[test]
    fn documented_xyzd_faces_triangle_tip_inward_in_each_quadrant() {
        let source = r#"
            function xyzd(t)
                local r=300
                local degxy=math.pi*t*2
                local x=r*math.sin(degxy)
                local y=r*math.cos(degxy)
                local z=0
                degxy=degxy-math.pi
                return x,y,z,degxy,0
            end
        "#;
        let motion =
            build_motion(Some(source), None, 3.0, 3.0, 120, 120, 30.0, 3.0, 1, None).unwrap();
        let config = particle_core::ParticleConfig {
            frequency: 40.0, // four births per second, one in each quadrant
            speed: 100.0,
            spread_degrees: 0.0,
            lifetime: 3.0,
            face_direction: true,
            script_motion: motion,
            ..Default::default()
        };
        let particles = particle_core::sample(&config, 1.0);
        for (id, expected) in [(0, 0.0), (1, 270.0), (2, 180.0), (3, 90.0)] {
            let particle = particles
                .iter()
                .find(|particle| particle.id == id)
                .unwrap_or_else(|| panic!("missing xyzd particle {id}"));
            let difference = (particle.rz as f64 - expected).rem_euclid(360.0);
            assert!(
                difference.min(360.0 - difference) < 0.001,
                "particle {id}: got {}, expected {expected}",
                particle.rz
            );
        }
    }

    #[test]
    fn documented_xyzd_degz_controls_z_output_direction() {
        let source = r#"
            function xyzd(t)
                return 0, 0, 0, 0, math.pi / 2
            end
        "#;
        let motion =
            build_motion(Some(source), None, 1.0, 1.0, 30, 30, 30.0, 1.0, 1, None).unwrap();
        assert!(motion.output_has_direction);
        let config = particle_core::ParticleConfig {
            frequency: 1.0,
            speed: 100.0,
            spread_degrees: 0.0,
            spread_z_degrees: 0.0,
            lifetime: 1.0,
            script_motion: motion,
            ..Default::default()
        };
        let particle = particle_core::sample(&config, 0.5)
            .into_iter()
            .find(|particle| particle.id == 0)
            .unwrap();
        assert!(particle.x.abs() < 0.001);
        assert!(particle.y.abs() < 0.001);
        assert!((particle.z - 50.0).abs() < 0.01);
        // degz is a Z output direction, not the triangle's Z roll. Roll is
        // controlled by rotxyz/degvxyz or the separate facing option.
        assert!(particle.rz.abs() < 0.001);
    }

    #[test]
    fn legacy_object_time_globals_are_available() {
        let source = r#"
            function xyz(t)
                return obj.totaltime - t, obj.framerate, obj.totalframe
            end
        "#;
        let motion =
            build_motion(Some(source), None, 2.0, 2.0, 60, 60, 30.0, 1.0, 1, None).unwrap();
        let endpoint = motion.output.last().unwrap();
        assert!(endpoint[0].abs() < 1e-9);
        assert_eq!(endpoint[1], 30.0);
        assert_eq!(endpoint[2], 60.0);
    }

    #[test]
    fn script_files_accept_utf8_bom_and_cp932() {
        assert_eq!(
            decode_script_bytes(b"\xef\xbb\xbffunction xyz(t) end").unwrap(),
            "function xyz(t) end"
        );
        assert_eq!(decode_script_bytes(&[0x82, 0xa0]).unwrap(), "あ");
    }

    #[test]
    fn every_bundled_custom_function_runs_to_its_endpoint() {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("templates/original/自作関数サンプル");
        let mut files = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("txt"))
            .collect::<Vec<_>>();
        files.sort();
        assert_eq!(files.len(), 11);

        let duration = 2.0;
        let (step, count) = curve_layout(duration);
        let host_values = HostValueTrace::new(
            1,
            step,
            (0..count)
                .map(|index| {
                    let time = index as f64 * step;
                    [time * 20.0, time * -10.0, time * 5.0]
                })
                .collect(),
        );

        let mut output_only = 0;
        let mut behavior_only = 0;
        let mut output_and_behavior = 0;
        for path in files {
            let source = load_source_file(&path).unwrap();
            let has_output = source.contains("function xyz(") || source.contains("function xyzd(");
            let has_behavior = source.contains("function vector(");
            assert!(has_output || has_behavior, "{}", path.display());
            let motion = build_motion(
                has_output.then_some(source.as_str()),
                has_behavior.then_some(source.as_str()),
                duration,
                duration,
                60,
                60,
                30.0,
                duration,
                1,
                Some(&host_values),
            )
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            if has_output {
                assert!(!motion.output.is_empty(), "{}", path.display());
                assert!(
                    motion
                        .output
                        .iter()
                        .flatten()
                        .all(|value| value.is_finite()),
                    "{}",
                    path.display()
                );
                if path.file_name().and_then(|name| name.to_str())
                    == Some("出力自作関数_擬似オブジェクト追跡.txt")
                {
                    let endpoint = motion.output.last().unwrap();
                    assert!((endpoint[0] - 40.0).abs() < 1e-9, "{}", path.display());
                    assert!((endpoint[1] + 20.0).abs() < 1e-9, "{}", path.display());
                    assert!((endpoint[2] - 10.0).abs() < 1e-9, "{}", path.display());
                }
            }
            if has_behavior {
                assert!(!motion.behavior_position.is_empty(), "{}", path.display());
                assert!(
                    motion
                        .behavior_position
                        .iter()
                        .flatten()
                        .all(|value| value.is_finite()),
                    "{}",
                    path.display()
                );
            }

            match (has_output, has_behavior) {
                (true, true) => output_and_behavior += 1,
                (true, false) => output_only += 1,
                (false, true) => behavior_only += 1,
                (false, false) => unreachable!(),
            }

            // Exercise the same final computation used when a triangle is the
            // source object. The renderer consumes x/y/z/rz from these samples.
            let triangle = particle_core::ParticleConfig {
                frequency: 1.0,
                speed: 100.0,
                spread_degrees: 0.0,
                initial_rotation_z_degrees: 0.0,
                rotation_z_degrees_per_second: 60.0,
                lifetime: duration,
                script_motion: motion,
                ..Default::default()
            };
            let particles = particle_core::sample(&triangle, duration * 0.75);
            let first = particles
                .iter()
                .find(|particle| particle.id == 0)
                .unwrap_or_else(|| panic!("{}: triangle particle is missing", path.display()));
            assert!(
                [first.x, first.y, first.z, first.rx, first.ry, first.rz]
                    .into_iter()
                    .all(f32::is_finite),
                "{}",
                path.display()
            );
            assert!((first.rz - 90.0).abs() < 0.001, "{}", path.display());
        }
        assert_eq!(output_only, 7);
        assert_eq!(behavior_only, 3);
        assert_eq!(output_and_behavior, 1);
    }
}
