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

pub(super) fn schema(index: usize) -> &'static LegacyEffectSchema {
    SCHEMAS[index - 1]
}

pub(super) fn config_items(index: usize) -> Vec<FilterConfigItem> {
    let schema = schema(index);
    let mut items = Vec::new();
    for track in schema.tracks {
        items.push(FilterConfigItem::Track(FilterConfigTrack {
            name: track.name.to_string(),
            value: track.default,
            range: track.min..=track.max,
            step: track.step,
            zero_display: None,
            slider_ratio: 1.0,
        }));
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
            items.push(FilterConfigItem::String(FilterConfigString {
                name: format!("{} ({})", dialog.name, dialog.key),
                value: dialog.default.to_string(),
            }));
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
        assert_eq!(schema(1).name, "パーティクル本体");
        assert_eq!(schema(32).name, "音");
    }

    #[test]
    fn dialog_labels_include_original_lua_variable() {
        let items = config_items(1);
        assert!(items.iter().any(|item| {
            matches!(item, FilterConfigItem::String(value) if value.name == "同時発生数 (sync)")
        }));
    }
}
