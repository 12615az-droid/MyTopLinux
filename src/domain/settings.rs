use serde::{Deserialize, Serialize};

pub const SUPPORTED_LANGUAGES: &[&str] = &[
    "", "en", "ru", "be", "uk", "de", "fr", "es", "it", "pt", "nl", "pl", "cs", "ro",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Appearance {
    pub language_tag: String,
    pub theme_mode: String,
    pub dynamic_colors: bool,
    pub accent_hex: String,
    pub font: String,
    pub text_scale: f64,
    pub corners: String,
    pub compact_cards: bool,
    pub show_ranks: bool,
    pub custom_font_path: Option<String>,
    pub custom_font_name: Option<String>,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            language_tag: String::new(),
            theme_mode: "SYSTEM".into(),
            dynamic_colors: true,
            accent_hex: "#6650A4".into(),
            font: "SYSTEM".into(),
            text_scale: 1.0,
            corners: "BALANCED".into(),
            compact_cards: false,
            show_ranks: true,
            custom_font_path: None,
            custom_font_name: None,
        }
    }
}

impl Appearance {
    pub fn sanitize(mut self) -> Self {
        if !SUPPORTED_LANGUAGES.contains(&self.language_tag.as_str()) {
            self.language_tag.clear();
        }
        let hex = self.accent_hex.trim().trim_start_matches('#');
        self.accent_hex = if hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            format!("#{}", hex.to_uppercase())
        } else {
            "#6650A4".into()
        };
        self.text_scale = if self.text_scale.is_finite() {
            self.text_scale.clamp(0.9, 1.2)
        } else {
            1.0
        };
        if !["SYSTEM", "LIGHT", "DARK"].contains(&self.theme_mode.as_str()) {
            self.theme_mode = "SYSTEM".into();
        }
        if !["SYSTEM", "SANS", "SERIF", "CONDENSED", "MONO", "CUSTOM"].contains(&self.font.as_str())
            || (self.font == "CUSTOM" && self.custom_font_path.is_none())
        {
            self.font = "SYSTEM".into();
        }
        if !["SUBTLE", "BALANCED", "SOFT"].contains(&self.corners.as_str()) {
            self.corners = "BALANCED".into();
        }
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SyncSettings {
    pub enabled: bool,
    pub server_url: String,
}

impl SyncSettings {
    pub fn normalize(mut self) -> Self {
        self.server_url = self.server_url.trim().trim_end_matches('/').into();
        self
    }
    pub fn valid_url(&self) -> bool {
        let Ok(url) = url::Url::parse(&self.server_url) else {
            return false;
        };
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && !self
                .server_url
                .split('/')
                .nth(2)
                .unwrap_or("")
                .contains('@')
            && url.query().is_none()
            && url.fragment().is_none()
            && url.port() != Some(0)
    }
    pub fn synchronize(&self) -> &'static str {
        if !self.enabled {
            "sync_disabled"
        } else if !self.valid_url() {
            "sync_invalid_url"
        } else {
            "sync_unavailable"
        }
    }
}
