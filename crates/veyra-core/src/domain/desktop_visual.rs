//! Persisted visual preferences only. Behaviour settings belong to P1-04B.
use super::{AppError, FieldPath};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum DesktopLanguage {
    #[default]
    #[serde(rename = "zh-CN")]
    SimplifiedChinese,
    #[serde(rename = "en-US")]
    English,
    #[serde(rename = "zh-TW")]
    TraditionalChinese,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

/// Content identity, never a path or data URL. Assets live in the injected asset root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct VisualAssetId(pub String);
impl VisualAssetId {
    pub fn is_valid(&self) -> bool {
        self.0.len() == 64
            && self
                .0
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }
}
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DesktopBackground {
    #[default]
    None,
    ManagedAsset(VisualAssetId),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopVisualPreferences {
    pub language: DesktopLanguage,
    pub theme_mode: DesktopThemeMode,
    pub background: DesktopBackground,
    pub background_opacity: u8,
    pub background_blur: u8,
    pub global_radius: u8,
    pub sidebar_collapsed: bool,
}
impl Default for DesktopVisualPreferences {
    fn default() -> Self {
        Self {
            language: DesktopLanguage::default(),
            theme_mode: DesktopThemeMode::System,
            background: DesktopBackground::None,
            background_opacity: 90,
            background_blur: 10,
            global_radius: 16,
            sidebar_collapsed: false,
        }
    }
}
impl DesktopVisualPreferences {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.background_opacity > 100
            || self.background_blur > 40
            || self.global_radius > 24
            || matches!(&self.background, DesktopBackground::ManagedAsset(id) if !id.is_valid())
        {
            return Err(AppError::validation(FieldPath::DesktopVisual));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_match_visual_source_and_all_enum_values_round_trip() {
        let p = DesktopVisualPreferences::default();
        assert_eq!(
            (p.background_opacity, p.background_blur, p.global_radius),
            (90, 10, 16)
        );
        assert!(!p.sidebar_collapsed);
        assert_eq!(p.theme_mode, DesktopThemeMode::System);
        for language in [
            DesktopLanguage::SimplifiedChinese,
            DesktopLanguage::English,
            DesktopLanguage::TraditionalChinese,
        ] {
            for theme_mode in [
                DesktopThemeMode::System,
                DesktopThemeMode::Light,
                DesktopThemeMode::Dark,
            ] {
                let value = DesktopVisualPreferences {
                    language,
                    theme_mode,
                    ..p.clone()
                };
                assert_eq!(
                    serde_json::from_value::<DesktopVisualPreferences>(
                        serde_json::to_value(&value).unwrap()
                    )
                    .unwrap(),
                    value
                );
            }
        }
    }
    #[test]
    fn rejects_out_of_range_and_non_managed_references() {
        for p in [
            DesktopVisualPreferences {
                background_opacity: 101,
                ..Default::default()
            },
            DesktopVisualPreferences {
                background_blur: 41,
                ..Default::default()
            },
            DesktopVisualPreferences {
                global_radius: 25,
                ..Default::default()
            },
            DesktopVisualPreferences {
                background: DesktopBackground::ManagedAsset(VisualAssetId("../image".into())),
                ..Default::default()
            },
        ] {
            assert!(p.validate().is_err());
        }
        for (field, value) in [("language", "fr"), ("theme_mode", "auto")] {
            let mut p = serde_json::to_value(DesktopVisualPreferences::default()).unwrap();
            p[field] = value.into();
            assert!(serde_json::from_value::<DesktopVisualPreferences>(p).is_err());
        }
    }
}
