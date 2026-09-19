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
        let prelude = format!(
            "os=nil;io=nil;package=nil;debug=nil;coroutine=nil;obj={{layer={},time={:.17},totaltime={:.17},frame={},totalframe={},framerate={:.17}}}\n",
            layer, object_time, object_total, frame, frame_total, frame_rate
        );
        vm.exec(&(prelude + source))?;
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

fn curve_layout(duration: f64) -> (f64, usize) {
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
        let motion =
            build_motion(Some(source), Some(source), 1.0, 2.0, 30, 60, 30.0, 1.0, 1).unwrap();
        let output = motion.output.last().unwrap();
        assert!((output[0] - 10.0).abs() < 1e-9);
        assert!((output[3] - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        let behavior = motion.behavior_position.last().unwrap();
        assert!((behavior[0] - 1.0).abs() < 1e-9);
        assert!((behavior[1] - 2.0).abs() < 1e-9);
        assert!((behavior[2] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn legacy_object_time_globals_are_available() {
        let source = r#"
            function xyz(t)
                return obj.totaltime - t, obj.framerate, obj.totalframe
            end
        "#;
        let motion = build_motion(Some(source), None, 2.0, 2.0, 60, 60, 30.0, 1.0, 1).unwrap();
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
}
