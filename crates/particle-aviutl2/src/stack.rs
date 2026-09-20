//! Layered AviUtl2 effects: one renderer at the bottom, independently
//! configurable extensions above it. Every extension DLL is a copy of the
//! same binary; its file stem selects the public effect name and controls.

use aviutl2::{
    AnyResult,
    filter::{FilterConfigItem, FilterPlugin, FilterPluginTable, FilterProcVideo},
};
use std::{collections::HashMap, ffi::c_void, path::PathBuf};

use crate::legacy_ui;
use crate::{FilterConfig, RenderOptions, render_filter_scripted};

pub(super) const BASIC_NAME: &str = "パーティクル本体";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Path,
    FrontBack,
    Direction,
    Output,
    Rotation,
    ScaleAlpha,
    Wind,
    Variation,
    Time,
    Time2,
    Convergence,
    Bounce,
    Mask,
    FilterMonochrome,
    Behavior,
    Dispersion,
    Orbit,
    Behavior4,
    Field,
    Trail,
    Image,
    Text,
    Video,
    Filter,
    Funnel,
    Mesh,
    Solid,
    Glass,
    CustomObject,
    OtherAnimationOption,
    Audio,
}

impl Kind {
    pub(super) const ALL: [Kind; 31] = [
        Self::Path,
        Self::FrontBack,
        Self::Direction,
        Self::Output,
        Self::Rotation,
        Self::Convergence,
        Self::ScaleAlpha,
        Self::Filter,
        Self::FilterMonochrome,
        Self::Variation,
        Self::Time,
        Self::Time2,
        Self::Bounce,
        Self::Mask,
        Self::Text,
        Self::Image,
        Self::Video,
        Self::Wind,
        Self::Behavior,
        Self::Dispersion,
        Self::Orbit,
        Self::Behavior4,
        Self::Field,
        Self::Trail,
        Self::Funnel,
        Self::Mesh,
        Self::Solid,
        Self::Glass,
        Self::CustomObject,
        Self::OtherAnimationOption,
        Self::Audio,
    ];

    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::FrontBack => "frontback",
            Self::Direction => "direction",
            Self::Output => "output",
            Self::Rotation => "rotation",
            Self::ScaleAlpha => "scale_alpha",
            Self::Wind => "wind",
            Self::Variation => "variation",
            Self::Time => "time",
            Self::Time2 => "time2",
            Self::Convergence => "convergence",
            Self::Bounce => "bounce",
            Self::Mask => "mask",
            Self::FilterMonochrome => "filter_monochrome",
            Self::Behavior => "behavior",
            Self::Dispersion => "dispersion",
            Self::Orbit => "orbit",
            Self::Behavior4 => "behavior4",
            Self::Field => "field",
            Self::Trail => "trail",
            Self::Image => "image",
            Self::Text => "text",
            Self::Video => "video",
            Self::Filter => "filter",
            Self::Funnel => "funnel",
            Self::Mesh => "mesh",
            Self::Solid => "solid",
            Self::Glass => "glass",
            Self::CustomObject => "custom_object",
            Self::OtherAnimationOption => "other_animation_option",
            Self::Audio => "audio",
        }
    }

    fn original_index(self) -> usize {
        match self {
            Self::Path => 2,
            Self::FrontBack => 3,
            Self::Direction => 4,
            Self::Output => 5,
            Self::Rotation => 6,
            Self::Convergence => 7,
            Self::ScaleAlpha => 8,
            Self::Filter => 9,
            Self::FilterMonochrome => 10,
            Self::Variation => 11,
            Self::Time => 12,
            Self::Time2 => 13,
            Self::Bounce => 14,
            Self::Mask => 15,
            Self::Text => 16,
            Self::Image => 17,
            Self::Video => 18,
            Self::Wind => 19,
            Self::Behavior => 20,
            Self::Dispersion => 21,
            Self::Orbit => 22,
            Self::Behavior4 => 23,
            Self::Field => 24,
            Self::Trail => 25,
            Self::Funnel => 26,
            Self::Mesh => 27,
            Self::Solid => 28,
            Self::Glass => 29,
            Self::CustomObject => 30,
            Self::OtherAnimationOption => 31,
            Self::Audio => 32,
        }
    }

    pub(super) fn name(self) -> &'static str {
        legacy_ui::schema(self.original_index()).name
    }

    fn registered_name(self) -> String {
        format!("{}{}", self.name(), crate::PARTICLE_SCRIPT_SUFFIX)
    }

    fn previous_registered_name(self) -> &'static str {
        // Keep projects created by beta.2 readable after the script suffix is
        // added. These were only used for the two names colliding with the
        // AviUtl2 standard effects.
        match self {
            Self::Rotation => "回転 [パーティクル(R)]",
            Self::Text => "テキスト [パーティクル(R)]",
            _ => "",
        }
    }

    fn article_chapter(self) -> u8 {
        match self {
            Self::Direction | Self::Path | Self::FrontBack => 3,
            Self::Output => 4,
            Self::Rotation | Self::Convergence | Self::ScaleAlpha => 5,
            Self::Filter | Self::FilterMonochrome => 6,
            Self::Variation | Self::Time | Self::Time2 => 7,
            Self::Bounce | Self::Mask => 8,
            Self::Text | Self::Image | Self::Video => 9,
            Self::Wind => 10,
            Self::Behavior | Self::Dispersion | Self::Orbit | Self::Behavior4 => 11,
            Self::Field | Self::Trail | Self::Funnel | Self::Mesh => 12,
            Self::Solid
            | Self::Glass
            | Self::CustomObject
            | Self::OtherAnimationOption
            | Self::Audio => 13,
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| {
            let previous = kind.previous_registered_name();
            kind.name() == name
                || kind.registered_name() == name
                || (!previous.is_empty() && previous == name)
        })
    }

    fn from_dll_path() -> Option<Self> {
        let stem = dll_path()?
            .file_stem()?
            .to_string_lossy()
            .to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|kind| stem == format!("particle_ext_{}", kind.key()))
    }
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleExW(flags: u32, address: *const u16, module: *mut *mut c_void) -> i32;
    fn GetModuleFileNameW(module: *mut c_void, path: *mut u16, capacity: u32) -> u32;
}

fn module_anchor() {}

fn dll_path() -> Option<PathBuf> {
    let mut module = std::ptr::null_mut();
    // FROM_ADDRESS | UNCHANGED_REFCOUNT. The address belongs to this DLL.
    let ok =
        unsafe { GetModuleHandleExW(0x6, module_anchor as *const () as *const u16, &mut module) };
    if ok == 0 {
        return None;
    }
    let mut buffer = vec![0u16; 32768];
    let length =
        unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    if length == 0 || length >= buffer.len() {
        return None;
    }
    Some(PathBuf::from(String::from_utf16_lossy(&buffer[..length])))
}

#[aviutl2::plugin(FilterPlugin)]
pub(super) struct StackBasicFilter;

impl FilterPlugin for StackBasicFilter {
    type Userdata = ();
    fn new(_info: aviutl2::AviUtl2Info) -> AnyResult<Self> {
        Ok(Self)
    }
    fn plugin_info(&self) -> FilterPluginTable {
        FilterPluginTable {
            name: format!("{}{}", BASIC_NAME, crate::PARTICLE_SCRIPT_SUFFIX),
            label: Some(crate::PARTICLE_LABEL.to_string()),
            information: "パーティクル(R) ver3.54B の本体名・設定名に合わせた移植。本体より上に拡張効果を追加します。"
                .to_string(),
            flags: aviutl2::bitflag!(aviutl2::filter::FilterPluginFlags { video: true }),
            config_items: legacy_ui::config_items(1),
        }
    }
    fn proc_video(
        &self,
        config: &[FilterConfigItem],
        video: &mut FilterProcVideo<()>,
    ) -> AnyResult<()> {
        let mut merged = FilterConfig::default();
        apply_basic(config, &mut merged);
        let end_at_object = item_check(config, "終了時に消える").unwrap_or(false);
        let script = apply_extensions(video, &mut merged);
        script.apply_resources(&mut merged, video.object.id);
        let output_source = script.resolve_source(script.output, video.object.id, "出力");
        let behavior_source = script.resolve_source(script.behavior, video.object.id, "挙動");
        render_filter_scripted(
            &merged,
            video,
            output_source.as_deref(),
            behavior_source.as_deref(),
            script.options,
            end_at_object,
        )
    }
}

#[aviutl2::plugin(FilterPlugin)]
pub(super) struct StackExtensionFilter {
    kind: Kind,
}

impl FilterPlugin for StackExtensionFilter {
    type Userdata = ();
    fn new(_info: aviutl2::AviUtl2Info) -> AnyResult<Self> {
        let kind = Kind::from_dll_path()
            .ok_or_else(|| std::io::Error::other("unknown particle extension DLL name"))?;
        Ok(Self { kind })
    }
    fn plugin_info(&self) -> FilterPluginTable {
        let items = legacy_ui::config_items(self.kind.original_index());
        FilterPluginTable {
            name: self.kind.registered_name(),
            label: Some(crate::PARTICLE_LABEL.to_string()),
            information: format!(
                "パーティクル(R) ver3.54B の拡張効果。{} より上に置きます。解説記事の第{}回と同じ効果名・設定名です。未移植の処理を含みます。",
                BASIC_NAME,
                self.kind.article_chapter()
            ),
            flags: aviutl2::bitflag!(aviutl2::filter::FilterPluginFlags { video: true }),
            config_items: items,
        }
    }
    fn proc_video(
        &self,
        _config: &[FilterConfigItem],
        _video: &mut FilterProcVideo<()>,
    ) -> AnyResult<()> {
        // The renderer below reads this effect's settings from the edit section.
        Ok(())
    }
}

fn item_track_value(items: &[FilterConfigItem], name: &str) -> Option<f64> {
    items.iter().find_map(|item| match item {
        FilterConfigItem::Track(track) if track.name == name => Some(track.value),
        FilterConfigItem::TrackGroup(group) => group
            .tracks
            .iter()
            .find(|track| track.name == name)
            .map(|track| track.value),
        _ => None,
    })
}

fn item_track(items: &[FilterConfigItem], name: &str) -> Option<i32> {
    item_track_value(items, name).map(|value| value as i32)
}

fn item_string<'a>(items: &'a [FilterConfigItem], name: &str) -> Option<&'a str> {
    items.iter().find_map(|item| match item {
        FilterConfigItem::String(value) if value.name == name => Some(value.value.as_str()),
        _ => None,
    })
}

fn item_check(items: &[FilterConfigItem], name: &str) -> Option<bool> {
    items.iter().find_map(|item| match item {
        FilterConfigItem::Check(check) if check.name == name => Some(check.value),
        _ => None,
    })
}

fn parse_number(value: &str) -> Option<f64> {
    let value = value.trim().trim_matches('"');
    if value.is_empty() || value.eq_ignore_ascii_case("nil") {
        None
    } else {
        value.parse().ok()
    }
}

fn parse_check(value: &str) -> bool {
    matches!(
        value.trim().trim_matches('"').to_ascii_lowercase().as_str(),
        "1" | "true" | "on"
    )
}

fn is_legacy_media_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "bmp"
            )
        })
}

fn parse_numbers(value: &str) -> Vec<f64> {
    value
        .trim()
        .trim_matches('"')
        .trim_matches(|ch| matches!(ch, '{' | '}' | '[' | ']'))
        .split(',')
        .filter_map(parse_number)
        .collect()
}

fn dialog_number(items: &[FilterConfigItem], label: &str, key: &str) -> Option<f64> {
    let name = format!("{label} ({key})");
    item_track_value(items, &name).or_else(|| item_string(items, &name).and_then(parse_number))
}

fn dialog_numbers(items: &[FilterConfigItem], label: &str, key: &str) -> Vec<f64> {
    let name = format!("{label} ({key})");
    item_track_value(items, &name)
        .map(|value| vec![value])
        .or_else(|| item_string(items, &name).map(parse_numbers))
        .unwrap_or_default()
}

fn legacy_xyz_z(values: &[f64]) -> Option<f64> {
    match values {
        [value] => Some(*value),
        [_, _, z, ..] => Some(*z),
        _ => None,
    }
}

fn apply_basic(items: &[FilterConfigItem], ui: &mut FilterConfig) {
    macro_rules! set {
        ($field:ident, $name:literal) => {
            if let Some(value) = item_track(items, $name) {
                ui.$field = value;
            }
        };
    }
    set!(speed, "出力速度");
    set!(frequency, "出力頻度");
    set!(direction, "出力方向");
    set!(spread, "拡散角度");
    if let Some(value) = dialog_number(items, "同時発生数", "sync") {
        ui.simultaneous = (value.round() as i32).max(1);
    }
    let gravity = dialog_numbers(items, "各xy重力", "grav");
    if gravity.len() >= 2 {
        ui.gravity_x = gravity[0].round() as i32;
        ui.gravity_y = gravity[1].round() as i32;
    }
    let rotation = dialog_numbers(items, "各xyz回転速度", "degvxyz");
    if let Some(value) = legacy_xyz_z(&rotation) {
        ui.rotation_z_speed = value.round() as i32;
    }
    let initial_rotation = dialog_numbers(items, "各xyz回転初期値", "rotxyz");
    if let Some(value) = legacy_xyz_z(&initial_rotation) {
        ui.rotation_z_initial = value.round() as i32;
    }
    if let Some(value) = dialog_number(items, "生存時間(秒", "ju") {
        ui.lifetime_centis = (value * 100.0).round() as i32;
    }
    let alpha = dialog_numbers(items, "透過率｛始、終｝", "palpha");
    if alpha.len() >= 2 {
        ui.alpha_start = alpha[0].round() as i32;
        ui.alpha_end = alpha[1].round() as i32;
    }
    let zoom = dialog_numbers(items, "拡大率｛始、終｝", "pzoom");
    if zoom.len() >= 2 {
        ui.zoom_start = zoom[0].round() as i32;
        ui.zoom_end = zoom[1].round() as i32;
    }
    if let Some(value) = dialog_number(items, "開始時間", "tsub") {
        ui.start_centis = (value * 100.0).round() as i32;
    }
    if let Some(value) = dialog_number(items, "ｼｰﾄﾞ", "ran") {
        ui.seed = value.round() as i32;
    }
}

#[derive(Clone, Copy)]
enum ScriptSource {
    Standard,
    Path(i32),
}

#[derive(Clone, Debug)]
struct PathEntry {
    path: PathBuf,
    single: bool,
}

#[derive(Default)]
struct ScriptSettings {
    standard_source: String,
    output_inline: String,
    behavior_inline: String,
    paths: HashMap<i32, PathEntry>,
    output: Option<ScriptSource>,
    behavior: Option<ScriptSource>,
    image_path: Option<(i32, bool)>,
    text_path: Option<(i32, bool)>,
    funnel_path: Option<i32>,
    solid_path: Option<i32>,
    options: RenderOptions,
}

fn append_script_source(destination: &mut String, source: String) {
    let source = crate::script_control::decode_effect_text(&source);
    if source.trim().is_empty() || destination.contains(source.trim()) {
        return;
    }
    if !destination.is_empty() {
        destination.push('\n');
    }
    destination.push_str(&source);
}

impl ScriptSettings {
    fn resolve_source(
        &self,
        source: Option<ScriptSource>,
        object_id: i64,
        label: &str,
    ) -> Option<String> {
        match source? {
            ScriptSource::Standard => {
                let inline = if label == "出力" {
                    &self.output_inline
                } else {
                    &self.behavior_inline
                };
                let mut result = self.standard_source.clone();
                append_script_source(&mut result, inline.clone());
                (!result.trim().is_empty()).then_some(result)
            }
            ScriptSource::Path(number) => {
                let Some(path) = self.paths.get(&number) else {
                    crate::p4_host::warn_once(
                        format!("script-path-{object_id}-{label}-{number}"),
                        format!("パーティクル(R): {label}が参照するパス番号{number}は未登録です"),
                    );
                    return None;
                };
                match crate::script_control::load_source_file(&path.path) {
                    Ok(source) => Some(source),
                    Err(error) => {
                        crate::p4_host::warn_once(
                            format!("script-file-{object_id}-{label}-{}", path.path.display()),
                            format!("パーティクル(R): {label}の外部スクリプト: {error}"),
                        );
                        None
                    }
                }
            }
        }
    }

    fn media_files(&self, number: i32, object_id: i64, label: &str) -> Vec<PathBuf> {
        let Some(entry) = self.paths.get(&number) else {
            crate::p4_host::warn_once(
                format!("material-path-{object_id}-{label}-{number}"),
                format!("パーティクル(R): {label}が参照するパス番号{number}は未登録です"),
            );
            return Vec::new();
        };
        if entry.single {
            return (entry.path.is_file() && is_legacy_media_path(&entry.path))
                .then(|| entry.path.clone())
                .into_iter()
                .collect();
        }
        let directory = if entry.path.is_dir() {
            entry.path.as_path()
        } else {
            entry.path.parent().unwrap_or(entry.path.as_path())
        };
        let mut files: Vec<_> = std::fs::read_dir(directory)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|item| item.path())
            .filter(|path| is_legacy_media_path(path))
            .take(4096)
            .collect();
        files.sort_by_key(|path| path.to_string_lossy().to_lowercase());
        files
    }

    fn apply_resources(&self, ui: &mut FilterConfig, object_id: i64) {
        if let Some((number, random)) = self.image_path {
            let files = self.media_files(number, object_id, "画像");
            if !files.is_empty() {
                ui.source_kind = 6;
                ui.image_files = files
                    .iter()
                    .map(|path| path.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("\n");
                ui.image_random = random;
            }
        }
        if let Some((number, lines)) = self.text_path {
            if let Some(entry) = self.paths.get(&number) {
                match crate::script_control::load_source_file(&entry.path) {
                    Ok(text) => {
                        ui.source_kind = if lines { 3 } else { 2 };
                        ui.source_text = text;
                    }
                    Err(error) => crate::p4_host::warn_once(
                        format!("text-path-{object_id}-{}", entry.path.display()),
                        format!("パーティクル(R): テキストの外部ファイル: {error}"),
                    ),
                }
            }
        }
        if let Some(number) = self.funnel_path {
            let files = self.media_files(number, object_id, "ファンネル");
            if !files.is_empty() {
                ui.funnel_image_files = files
                    .iter()
                    .map(|path| path.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("\n");
                ui.funnel_image_random = true;
            }
        }
        if let Some(number) = self.solid_path {
            if ui.solid_shape == 6 {
                let detail = self
                    .paths
                    .get(&number)
                    .map(|entry| entry.path.display().to_string())
                    .unwrap_or_else(|| format!("パス番号{number}"));
                crate::p4_host::warn_once(
                    format!("pmd-path-{object_id}-{number}"),
                    format!(
                        "パーティクル(R): 立体物タイプ6の旧PMD .datはbetaでは読込対象外です: {detail}"
                    ),
                );
                ui.source_kind = 0;
            }
        }
    }
}

fn apply_extensions(video: &mut FilterProcVideo<()>, ui: &mut FilterConfig) -> ScriptSettings {
    let mut script = ScriptSettings::default();
    let Some(object) = video.get_image_object(video.object.layer, 0.0) else {
        return script;
    };
    let frame = video.object.frame as f64;
    let section = video.read_section();
    let Ok(effects) = section.get_effects(object) else {
        return script;
    };
    // The edit section lists effects in display order. The first renderer is
    // the bottom boundary of the extension stack for this particle object.
    // Standard Script Control is different: it is a host effect whose source
    // is shared by the output/behavior adapters, so its placement must not
    // make the source disappear. Keep scanning after the particle renderer
    // for that source, while retaining the renderer boundary for extensions.
    let mut extensions_above_basic = true;
    for effect in effects {
        let Ok(name) = section.get_effect_name(effect) else {
            continue;
        };
        if name.contains("スクリプト制御") && section.get_effect_enable(effect).ok() == Some(true)
        {
            for item in ["スクリプト", "コード", "script"] {
                if let Ok(source) = section.get_effect_item_value(effect, item) {
                    append_script_source(&mut script.standard_source, source);
                    break;
                }
            }
        }
        if name == BASIC_NAME || name == format!("{}{}", BASIC_NAME, crate::PARTICLE_SCRIPT_SUFFIX)
        {
            extensions_above_basic = false;
            continue;
        }
        if !extensions_above_basic {
            continue;
        }
        let Some(kind) = Kind::from_name(&name) else {
            continue;
        };
        if section.get_effect_enable(effect).ok() != Some(true) {
            continue;
        }
        apply_kind(section, effect, frame, kind, ui, &mut script);
    }
    script
}

fn effect_dialog_text(
    section: &aviutl2::generic::ReadSection,
    effect: aviutl2::generic::EffectHandle,
    label: &str,
    key: &str,
) -> Option<String> {
    section
        .get_effect_item_value(effect, &format!("{label} ({key})"))
        .ok()
}

fn effect_dialog_numbers(
    section: &aviutl2::generic::ReadSection,
    effect: aviutl2::generic::EffectHandle,
    frame: f64,
    label: &str,
    key: &str,
) -> Vec<f64> {
    let name = format!("{label} ({key})");
    section
        .get_effect_track_value(effect, &name, frame)
        .ok()
        .filter(|value| value.is_finite())
        .map(|value| vec![value])
        .or_else(|| {
            section
                .get_effect_item_value(effect, &name)
                .ok()
                .map(|value| parse_numbers(&value))
        })
        .unwrap_or_default()
}

fn effect_dialog_number(
    section: &aviutl2::generic::ReadSection,
    effect: aviutl2::generic::EffectHandle,
    frame: f64,
    label: &str,
    key: &str,
) -> Option<f64> {
    effect_dialog_numbers(section, effect, frame, label, key)
        .into_iter()
        .next()
}

fn effect_dialog_check(
    section: &aviutl2::generic::ReadSection,
    effect: aviutl2::generic::EffectHandle,
    frame: f64,
    label: &str,
    key: &str,
) -> bool {
    effect_dialog_number(section, effect, frame, label, key).map_or_else(
        || effect_dialog_text(section, effect, label, key).is_some_and(|value| parse_check(&value)),
        |value| value != 0.0,
    )
}

fn effect_item_check(
    section: &aviutl2::generic::ReadSection,
    effect: aviutl2::generic::EffectHandle,
    frame: f64,
    name: &str,
) -> bool {
    section
        .get_effect_track_value(effect, name, frame)
        .ok()
        .filter(|value| value.is_finite())
        .is_some_and(|value| value.round() != 0.0)
        || section
            .get_effect_item_value(effect, name)
            .ok()
            .is_some_and(|value| parse_check(&value))
}

fn apply_kind(
    section: &aviutl2::generic::ReadSection,
    effect: aviutl2::generic::EffectHandle,
    frame: f64,
    kind: Kind,
    ui: &mut FilterConfig,
    script: &mut ScriptSettings,
) {
    macro_rules! set {
        ($field:ident, $name:literal) => {
            if let Ok(value) = section.get_effect_track_value(effect, $name, frame) {
                if value.is_finite() {
                    ui.$field = value.round() as i32;
                }
            }
        };
    }
    match kind {
        Kind::Path => {
            let number = section
                .get_effect_track_value(effect, "パス番号", frame)
                .ok()
                .filter(|value| value.is_finite())
                .map(|value| value.round() as i32)
                .unwrap_or(0);
            if (1..=10).contains(&number)
                && let Ok(value) = section.get_effect_item_value(effect, "ファイル")
            {
                let path = value.trim().trim_matches('"');
                if !path.trim().is_empty() {
                    let single = section
                        .get_effect_item_value(effect, "単体")
                        .ok()
                        .is_some_and(|value| parse_check(&value));
                    script.paths.insert(
                        number,
                        PathEntry {
                            path: PathBuf::from(path),
                            single,
                        },
                    );
                }
            }
        }
        Kind::FrontBack => {
            script.options.reverse_draw_order = effect_item_check(section, effect, frame, "前⇔後");
            script.options.face_mode = effect_item_check(section, effect, frame, "表裏合成") as i32;
            script.options.inverse_frequency =
                effect_dialog_check(section, effect, frame, "頻度逆転/chk", "ref");
            script.options.reverse_time =
                effect_dialog_check(section, effect, frame, "逆再生/chk", "rep");
            script.options.face_direction =
                effect_dialog_check(section, effect, frame, "進行方向を向く/chk", "prog");
            set!(rotation_order, "回転表現");
        }
        Kind::Direction => {
            set!(direction_z, "z出力方向");
            set!(spread_z, "z拡散角度");
            set!(gravity_z, "z重力");
        }
        Kind::Output => {
            let output_type = section
                .get_effect_track_value(effect, "出力ﾀｲﾌﾟ", frame)
                .ok()
                .filter(|value| value.is_finite())
                .map(|value| value.round() as i32)
                .unwrap_or(0);
            if output_type != 0 {
                ui.shape = match output_type {
                    3 => 1,
                    4 => 2,
                    5 => 3,
                    _ => 0,
                };
            }
            let option = effect_item_check(section, effect, frame, "ｵﾌﾟｼｮﾝ") as i32;
            script.output = (output_type == 8).then(|| {
                if option == 0 {
                    let number = section
                        .get_effect_track_value(effect, "数", frame)
                        .ok()
                        .filter(|value| value.is_finite())
                        .map(|value| value.round() as i32)
                        .unwrap_or(0);
                    ScriptSource::Path(number)
                } else {
                    ScriptSource::Standard
                }
            });
            if output_type != 8 {
                set!(simultaneous, "数");
            }
            if let Ok(value) = section.get_effect_track_value(effect, "値", frame) {
                let value = value.round() as i32;
                ui.extent_x = value;
                ui.extent_y = value;
                ui.extent_z = value;
            }
            if let Ok(source) = section.get_effect_item_value(effect, legacy_ui::SCRIPT_CODE_ITEM) {
                append_script_source(&mut script.output_inline, source);
            }
        }
        Kind::Rotation => {
            set!(rotation_z_speed, "揺れ速度");
        }
        Kind::ScaleAlpha => {
            set!(zoom_start, "始拡大率");
            set!(zoom_end, "終拡大率");
            set!(alpha_start, "始透過率");
            set!(alpha_end, "終透過率");
        }
        Kind::Wind => {
            // The original graph-based wind format needs the P5 graph evaluator.
        }
        Kind::Variation => {
            let values = effect_dialog_numbers(section, effect, frame, "速度", "bv");
            if let Some(value) = values.first() {
                ui.variation_speed = value.abs().round() as i32;
            }
        }
        Kind::Time => {
            set!(wave_speed, "1速度");
            set!(wave_speed_period, "周期ms");
        }
        Kind::Time2 => {
            // Original graph selection is retained in the UI and documented as pending.
        }
        Kind::Convergence => {
            ui.converge_enabled = true;
            set!(converge_x, "x座標");
            set!(converge_y, "y座標");
            set!(converge_z, "z座標");
            if let Some(value) =
                effect_dialog_number(section, effect, frame, "集結開始時間ms", "contime")
            {
                ui.converge_start_ms = value.round() as i32;
            }
        }
        Kind::Bounce => {
            ui.bounce_enabled = true;
            let x = effect_dialog_numbers(section, effect, frame, "x範囲", "xrange");
            let y = effect_dialog_numbers(section, effect, frame, "y範囲", "yrange");
            let z = effect_dialog_numbers(section, effect, frame, "z範囲", "zrange");
            if x.len() >= 2 {
                ui.bounce_x_min = x[0].round() as i32;
                ui.bounce_x_max = x[1].round() as i32;
            }
            if y.len() >= 2 {
                ui.bounce_y_min = y[0].round() as i32;
                ui.bounce_y_max = y[1].round() as i32;
            }
            if z.len() >= 2 {
                ui.bounce_z_min = z[0].round() as i32;
                ui.bounce_z_max = z[1].round() as i32;
            }
            let restitution = effect_dialog_numbers(section, effect, frame, "xyz反発係数", "ex");
            if let Some(value) = restitution.first() {
                ui.bounce_restitution = (value * 100.0).round() as i32;
            }
        }
        Kind::Mask => {
            let layers = effect_dialog_numbers(section, effect, frame, "ﾚｲﾔｰ", "relayer");
            if let Some(value) = layers.first() {
                ui.mask_layer = value.round() as i32;
            }
            let restitution = effect_dialog_numbers(section, effect, frame, "各反発係数", "ree");
            if let Some(value) = restitution.first() {
                ui.mask_restitution = (value * 100.0).round() as i32;
            }
            ui.mask_mode = 3;
        }
        Kind::Behavior => {
            let own_function =
                effect_dialog_check(section, effect, frame, "*自作関数/chk", "orifunc");
            let script_source =
                effect_dialog_check(section, effect, frame, "*ｽｸﾘﾌﾟﾄ制御記述/chk", "scriptf");
            script.behavior = if own_function {
                if script_source {
                    Some(ScriptSource::Standard)
                } else {
                    let number =
                        effect_dialog_number(section, effect, frame, "*取得ﾊﾟｽ番号", "pathn")
                            .map(|value| value.round() as i32)
                            .unwrap_or(0);
                    (1..=10)
                        .contains(&number)
                        .then_some(ScriptSource::Path(number))
                }
            } else {
                None
            };
            if let Ok(source) = section.get_effect_item_value(effect, legacy_ui::SCRIPT_CODE_ITEM) {
                append_script_source(&mut script.behavior_inline, source);
            }
        }
        Kind::FilterMonochrome
        | Kind::Behavior4
        | Kind::Field
        | Kind::CustomObject
        | Kind::OtherAnimationOption => {}
        Kind::Dispersion => {
            ui.disperse_enabled = true;
            set!(disperse_after_ms, "分散ﾀｲﾑms");
            set!(disperse_xy, "xy拡散度");
            set!(disperse_z, "z拡散度");
            set!(stop_after_ms, "急停止ms");
        }
        Kind::Orbit => {
            ui.orbit_mode = 1;
            set!(orbit_x, "中心x");
            set!(orbit_y, "中心y");
            set!(orbit_z, "中心z");
            set!(orbit_angular_speed, "角速度");
            if let Some(value) = effect_dialog_number(section, effect, frame, "半径速度", "hd")
            {
                ui.orbit_radial_speed = value.round() as i32;
            }
        }
        Kind::Trail => {
            set!(trail_length_ms, "消失(ﾐﾘ秒");
            set!(trail_width, "尻尾");
            set!(trail_samples, "通過点数");
            if let Ok(value) = section.get_effect_track_value(effect, "軌跡ﾀｲﾌﾟ", frame) {
                ui.trail_mode = (value.round() as i32 + 1).clamp(1, 3);
            }
        }
        Kind::Image => {
            ui.source_kind = 0;
            let number = section
                .get_effect_track_value(effect, "ﾊﾟｽ番号", frame)
                .ok()
                .filter(|value| value.is_finite())
                .map(|value| value.round() as i32)
                .unwrap_or(0);
            let random = effect_item_check(section, effect, frame, "ぱらばら");
            script.image_path = (1..=10).contains(&number).then_some((number, random));
        }
        Kind::Text => {
            let lines = section
                .get_effect_item_value(effect, "出力を一文字から行に")
                .ok()
                .is_some_and(|value| parse_check(&value));
            ui.source_kind = if lines { 3 } else { 2 };
            if let Some(value) = effect_dialog_text(section, effect, "テキスト", "pstr") {
                ui.source_text = value.trim_matches('"').to_string();
            }
            let number = effect_dialog_number(section, effect, frame, "ﾊﾟｽ取得0~10", "gpath")
                .map(|value| value.round() as i32)
                .unwrap_or(0);
            script.text_path = (1..=10).contains(&number).then_some((number, lines));
        }
        Kind::Video => {
            ui.source_kind = 1;
        }
        Kind::Filter => {
            if let Some(value) = effect_dialog_text(section, effect, "ﾌｨﾙﾀｰ名", "nae") {
                ui.particle_effect_name = value.trim_matches('"').to_string();
            }
        }
        Kind::Funnel => {
            set!(funnel_count, "数");
            set!(funnel_scale, "ｻｲｽﾞ");
            set!(funnel_radius, "半径");
            set!(funnel_rings, "円環数");
            if let Some(value) = effect_dialog_number(section, effect, frame, "公転速度", "revo")
            {
                ui.funnel_angular_speed = value.round() as i32;
            }
            if let Some(value) = effect_dialog_text(section, effect, "図形/fig", "fig") {
                let value = value.trim_matches('"');
                if let Some(position) = value.to_ascii_lowercase().find("path") {
                    let number: String = value[position + 4..]
                        .chars()
                        .take_while(char::is_ascii_digit)
                        .collect();
                    script.funnel_path = number
                        .parse::<i32>()
                        .ok()
                        .filter(|number| (1..=10).contains(number));
                }
            }
            if let Some(value) = effect_dialog_number(section, effect, frame, "自転速度", "rot")
            {
                ui.funnel_self_spin = value.round() as i32;
            }
        }
        Kind::Mesh => {
            ui.mesh_enabled = true;
            set!(mesh_alpha, "透過率");
        }
        Kind::Solid => {
            ui.source_kind = 4;
            set!(solid_shape, "タイプ");
            set!(solid_size, "大きさ");
            set!(solid_divisions, "分割数");
            if let Some(value) =
                effect_dialog_number(section, effect, frame, "ﾀｲﾌﾟ4横曲率", "kyoku1")
            {
                ui.solid_curve_x = value.round() as i32;
            }
            if let Some(value) =
                effect_dialog_number(section, effect, frame, "ﾀｲﾌﾟ4縦曲率", "kyoku2")
            {
                ui.solid_curve_y = value.round() as i32;
            }
            if let Some(value) = effect_dialog_number(section, effect, frame, "ﾀｲﾌﾟ5奥行き", "oku")
            {
                ui.solid_depth = value.round() as i32;
            }
            let number = section
                .get_effect_track_value(effect, "パス番号", frame)
                .ok()
                .filter(|value| value.is_finite())
                .map(|value| value.round() as i32)
                .unwrap_or(0);
            script.solid_path = (1..=10).contains(&number).then_some(number);
        }
        Kind::Glass => {
            ui.source_kind = 5;
            set!(solid_size, "倍率");
        }
        Kind::Audio => {
            set!(audio_band, "取得位置");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_original_extension_has_a_unique_plugin() {
        assert_eq!(Kind::from_name("パス@particle2r"), Some(Kind::Path));
        assert_eq!(Kind::from_name("パス"), Some(Kind::Path));
        assert_eq!(
            Kind::from_name("回転 [パーティクル(R)]"),
            Some(Kind::Rotation)
        );
        let mut names = std::collections::HashSet::new();
        let mut keys = std::collections::HashSet::new();
        for kind in Kind::ALL {
            assert!(names.insert(kind.name()));
            assert!(keys.insert(kind.key()));
            assert!((3..=13).contains(&kind.article_chapter()));
            let info = StackExtensionFilter { kind }.plugin_info();
            assert_eq!(info.label.as_deref(), Some(crate::PARTICLE_LABEL));
            assert_eq!(info.name, kind.registered_name());
            assert!(info.name.ends_with(crate::PARTICLE_SCRIPT_SUFFIX));
            let items = info.config_items;
            assert!(!items.is_empty(), "{} has no controls", kind.name());
            assert_eq!(kind.name(), legacy_ui::schema(kind.original_index()).name);
        }
        assert_eq!(names.len(), 31);
        let basic_info = StackBasicFilter.plugin_info();
        assert_eq!(basic_info.label.as_deref(), Some(crate::PARTICLE_LABEL));
        assert_eq!(
            basic_info.name,
            format!("{}{}", BASIC_NAME, crate::PARTICLE_SCRIPT_SUFFIX)
        );
        let basic = basic_info.config_items;
        assert!(item_track(&basic, "出力速度").is_some());
        assert_eq!(item_check(&basic, "終了時に消える"), Some(false));
        let mut config = FilterConfig::default();
        apply_basic(&basic, &mut config);
        assert_eq!(config.speed, 100);
        assert_eq!(config.lifetime_centis, 300);
        assert_eq!(config.alpha_end, 50);
        assert_eq!(config.rotation_z_initial, 0);
        assert_eq!(config.rotation_z_speed, 60);
    }

    #[test]
    fn basic_rotation_accepts_legacy_xyz_tables_and_single_values() {
        let mut basic = StackBasicFilter.plugin_info().config_items;
        for item in &mut basic {
            if let FilterConfigItem::String(value) = item {
                match value.name.as_str() {
                    "各xyz回転初期値 (rotxyz)" => value.value = "{0,0,25}".to_string(),
                    "各xyz回転速度 (degvxyz)" => value.value = "-120".to_string(),
                    _ => {}
                }
            }
        }
        let mut config = FilterConfig::default();
        apply_basic(&basic, &mut config);
        assert_eq!(config.rotation_z_initial, 25);
        assert_eq!(config.rotation_z_speed, -120);
    }

    #[test]
    fn path_media_scope_matches_beta_image_loader() {
        assert!(is_legacy_media_path(std::path::Path::new("素材.PNG")));
        assert!(is_legacy_media_path(std::path::Path::new("素材.jpeg")));
        assert!(!is_legacy_media_path(std::path::Path::new("素材.txt")));
        assert!(!is_legacy_media_path(std::path::Path::new("素材.mp4")));
    }
}
