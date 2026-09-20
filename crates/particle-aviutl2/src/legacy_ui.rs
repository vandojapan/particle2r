//! UI definitions generated from the original ver3.54B script headers.
//!
//! AviUtl2 expands old `--dialog` values into individual settings instead of
//! opening the AviUtl 1.x dialog.  We keep the original label and Lua variable
//! name together so the bundled documentation and script source can be read
//! side by side.

use aviutl2::filter::{
    FilterConfigCheck, FilterConfigFile, FilterConfigGroup, FilterConfigItem, FilterConfigString,
    FilterConfigText, FilterConfigTrack,
};

pub(super) const SCRIPT_CODE_ITEM: &str = "スクリプト制御コード（取得できない場合）";

pub(super) struct LegacyTrack {
    pub name: &'static str,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub step: f64,
}

pub(super) struct LegacyCheck {
    pub name: &'static str,
    pub default: bool,
}

pub(super) struct LegacyDialog {
    pub name: &'static str,
    pub key: &'static str,
    pub default: &'static str,
}

pub(super) struct LegacyEffectSchema {
    pub name: &'static str,
    pub tracks: &'static [LegacyTrack],
    pub check: Option<LegacyCheck>,
    pub dialogs: &'static [LegacyDialog],
    pub file_input: bool,
}

include!("legacy_ui_data.rs");

const DIALOG_TRACK_LIMIT: f64 = 4096.0;

fn enum_maximum(name: &str) -> Option<f64> {
    ["0~", "0～"].into_iter().find_map(|marker| {
        let start = name.find(marker)? + marker.len();
        let digits = name[start..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>();
        (!digits.is_empty())
            .then(|| digits.parse::<f64>().ok())
            .flatten()
    })
}

fn legacy_track_item(track: &LegacyTrack) -> FilterConfigItem {
    if track.min == 0.0 && track.max == 1.0 {
        return FilterConfigItem::Check(FilterConfigCheck {
            name: track.name.to_string(),
            value: track.default != 0.0,
        });
    }
    FilterConfigItem::Track(FilterConfigTrack {
        name: track.name.to_string(),
        value: track.default,
        range: track.min..=track.max,
        step: track.step,
        zero_display: None,
        slider_ratio: 1.0,
    })
}

fn dialog_item(dialog: &LegacyDialog) -> Option<FilterConfigItem> {
    let value = dialog.default.trim().parse::<f64>().ok()?;
    if !value.is_finite() {
        return None;
    }
    let name = format!("{} ({})", dialog.name, dialog.key);
    let integer = value.fract().abs() < f64::EPSILON;
    if dialog.name.contains("/chk") {
        return Some(FilterConfigItem::Check(FilterConfigCheck {
            name,
            value: value != 0.0,
        }));
    }
    let (range, slider_ratio) = if let Some(maximum) = enum_maximum(dialog.name) {
        (0.0..=maximum.max(value), 1.0)
    } else {
        // `--dialog` had no range declaration. Keep a generous accepted range,
        // while slider_ratio limits ordinary mouse operation to a useful span.
        (-DIALOG_TRACK_LIMIT..=DIALOG_TRACK_LIMIT, 0.001)
    };
    Some(FilterConfigItem::Track(FilterConfigTrack {
        name,
        value,
        range,
        step: if integer { 1.0 } else { 0.01 },
        zero_display: None,
        slider_ratio,
    }))
}

pub(super) fn schema(index: usize) -> &'static LegacyEffectSchema {
    SCHEMAS[index - 1]
}

pub(super) fn config_items(index: usize) -> Vec<FilterConfigItem> {
    let schema = schema(index);
    let mut items = Vec::new();
    for track in schema.tracks {
        items.push(legacy_track_item(track));
    }
    if let Some(check) = &schema.check {
        items.push(FilterConfigItem::Check(FilterConfigCheck {
            name: check.name.to_string(),
            value: check.default,
        }));
    }
    if schema.file_input {
        items.push(FilterConfigItem::File(FilterConfigFile {
            name: "ファイル".to_string(),
            value: String::new(),
            filters: Vec::new(),
        }));
    }
    if !schema.dialogs.is_empty() {
        items.push(FilterConfigItem::Group(
            FilterConfigGroup::start_with_opened("詳細設定（旧版の「設定」）".to_string(), false),
        ));
        for dialog in schema.dialogs {
            if let Some(item) = dialog_item(dialog) {
                items.push(item);
            } else {
                items.push(FilterConfigItem::String(FilterConfigString {
                    name: format!("{} ({})", dialog.name, dialog.key),
                    value: dialog.default.to_string(),
                }));
            }
        }
        items.push(FilterConfigItem::Group(FilterConfigGroup::end()));
    }
    if matches!(index, 5 | 20) {
        items.push(FilterConfigItem::Text(FilterConfigText {
            name: SCRIPT_CODE_ITEM.to_string(),
            value: String::new(),
        }));
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_schema_matches_original_inventory() {
        assert_eq!(SCHEMAS.len(), 32);
        assert_eq!(
            SCHEMAS.iter().map(|item| item.tracks.len()).sum::<usize>(),
            104
        );
        let rendered = (1..=SCHEMAS.len()).flat_map(config_items).fold(
            (0, 0, 0),
            |(tracks, checks, strings), item| match item {
                FilterConfigItem::Track(_) => (tracks + 1, checks, strings),
                FilterConfigItem::Check(_) => (tracks, checks + 1, strings),
                FilterConfigItem::String(_) => (tracks, checks, strings + 1),
                _ => (tracks, checks, strings),
            },
        );
        assert_eq!(rendered, (209, 112, 147));
        assert_eq!(schema(1).name, "パーティクル本体");
        assert_eq!(schema(32).name, "音");
    }

    #[test]
    fn numeric_dialogs_are_tracks_and_keep_original_lua_variable() {
        let numeric_dialogs = SCHEMAS
            .iter()
            .flat_map(|schema| schema.dialogs)
            .filter(|dialog| dialog_item(dialog).is_some())
            .count();
        assert_eq!(numeric_dialogs, 188);
        let checkbox_dialogs = SCHEMAS
            .iter()
            .flat_map(|schema| schema.dialogs)
            .filter(|dialog| dialog.name.contains("/chk") && dialog_item(dialog).is_some())
            .count();
        assert_eq!(checkbox_dialogs, 68);

        let items = config_items(1);
        assert!(items.iter().any(|item| {
            matches!(item, FilterConfigItem::Track(value)
                if value.name == "同時発生数 (sync)" && value.value == 0.0)
        }));
        assert!(items.iter().any(|item| {
            matches!(item, FilterConfigItem::Track(value)
                if value.name == "加速度 (ac)"
                    && value.range == (-4096.0..=4096.0))
        }));
        assert!(items.iter().any(|item| {
            matches!(item, FilterConfigItem::String(value)
                if value.name == "各xy重力 (grav)" && value.value == "{0,0}")
        }));
    }

    #[test]
    fn binary_legacy_controls_are_checks() {
        let binary_names = SCHEMAS
            .iter()
            .flat_map(|schema| schema.tracks)
            .filter(|track| track.min == 0.0 && track.max == 1.0)
            .map(|track| track.name)
            .collect::<std::collections::HashSet<_>>();
        assert!(!binary_names.is_empty());
        let all_items = (1..=SCHEMAS.len())
            .flat_map(config_items)
            .collect::<Vec<_>>();
        for name in binary_names {
            assert!(all_items.iter().any(|item| {
                matches!(item, FilterConfigItem::Check(value) if value.name == name)
            }));
            assert!(!all_items.iter().any(|item| {
                matches!(item, FilterConfigItem::Track(value) if value.name == name)
            }));
        }
        assert!(
            SCHEMAS
                .iter()
                .flat_map(|schema| schema.dialogs)
                .any(|dialog| dialog.name.contains("/chk")
                    && all_items.iter().any(|item| {
                        matches!(item, FilterConfigItem::Check(value)
                    if value.name == format!("{} ({})", dialog.name, dialog.key))
                    }))
        );
    }
}
