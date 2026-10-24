use crate::error::AppResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    #[default]
    Playing,
    Completed,
    Replaying,
    Dropped,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Playing => "playing",
            Self::Completed => "completed",
            Self::Replaying => "replaying",
            Self::Dropped => "dropped",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    #[serde(default)]
    pub id: i64,
    pub title: String,
    pub completions: i32,
    pub status: Status,
    pub rating: f64,
    #[serde(default)]
    pub cover_uri: Option<String>,
    #[serde(default)]
    pub completion_dates: Vec<String>,
    #[serde(default)]
    pub series: Option<String>,
    #[serde(default)]
    pub universe: Option<String>,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub developers: Vec<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub release_year: Option<i32>,
    #[serde(default)]
    pub opinion: Option<String>,
    #[serde(default)]
    pub playtime_hours: Option<f64>,
}

impl Game {
    pub fn validate(&self) -> AppResult<()> {
        if self.title.trim().is_empty() {
            return Err("title_needed".into());
        }
        if !self.rating.is_finite() || !(0.0..=10.0).contains(&self.rating) {
            return Err("rating: 0–10".into());
        }
        if self.completions < 0 {
            return Err("completions: ≥ 0".into());
        }
        if self
            .playtime_hours
            .is_some_and(|hours| !hours.is_finite() || hours < 0.0)
        {
            return Err("playtimeHours: ≥ 0".into());
        }
        Ok(())
    }

    pub fn normalized(mut self) -> AppResult<Self> {
        self.validate()?;
        self.title = self.title.trim().into();
        Ok(self)
    }
}
