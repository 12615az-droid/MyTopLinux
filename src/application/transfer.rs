use crate::{
    domain::{average, Game, Query},
    error::AppResult,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const MAX_IMPORT_BYTES: usize = 10 * 1024 * 1024;
pub const STRINGS: &str = include_str!("../../ui/strings.json");

pub fn decode_import(text: &str) -> AppResult<Vec<Game>> {
    if text.len() > MAX_IMPORT_BYTES {
        return Err("import_limits".into());
    }
    let root: Value = serde_json::from_str(text).map_err(|_| "import_error")?;
    let records = if root.is_array() {
        root
    } else {
        if root.get("version").unwrap_or(&json!(1)) != &json!(1) {
            return Err("Unsupported JSON version".into());
        }
        root.get("games").cloned().ok_or("import_error")?
    };
    let records = records.as_array().ok_or("import_error")?;
    if records.len() > 10_000 {
        return Err("import_limits".into());
    }
    records
        .iter()
        .map(|value| {
            let mut game: Game =
                serde_json::from_value(value.clone()).map_err(|_| "import_error")?;
            game.id = 0;
            game.normalized()
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExportOptions {
    pub format: String,
    pub filtered: bool,
    pub include_metadata: bool,
    pub include_history: bool,
    pub include_opinion: bool,
    pub include_cover: bool,
    pub top_only: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: "JSON".into(),
            filtered: false,
            include_metadata: true,
            include_history: true,
            include_opinion: true,
            include_cover: true,
            top_only: false,
        }
    }
}

pub fn export(
    games: &[Game],
    query: &Query,
    options: &ExportOptions,
    language: &str,
) -> AppResult<String> {
    let query = if options.filtered {
        query.clone()
    } else {
        Query {
            sort: query.sort,
            ..Default::default()
        }
    };
    let games = query.apply(games);
    if options.format == "JSON" {
        let mut values = Vec::new();
        for game in &games {
            let mut value = serde_json::to_value(game).map_err(|error| error.to_string())?;
            let object = value.as_object_mut().ok_or("export_error")?;
            let mut omitted = Vec::new();
            if !options.include_metadata {
                omitted.extend([
                    "series",
                    "universe",
                    "genres",
                    "tags",
                    "developers",
                    "platforms",
                    "releaseYear",
                ]);
            }
            if !options.include_history {
                omitted.extend(["completionDates", "playtimeHours"]);
            }
            if !options.include_opinion {
                omitted.push("opinion");
            }
            if !options.include_cover {
                omitted.push("coverUri");
            }
            for key in omitted {
                object.remove(key);
            }
            values.push(value);
        }
        return serde_json::to_string_pretty(&json!({"version": 1, "games": values}))
            .map_err(|error| error.to_string());
    }
    export_text(&games, options, language)
}

fn export_text(games: &[Game], options: &ExportOptions, language: &str) -> AppResult<String> {
    let strings: Value = serde_json::from_str(STRINGS).map_err(|error| error.to_string())?;
    let lang = strings.get(language).unwrap_or(&strings["en"]);
    let label = |key: &str| {
        lang[key]
            .as_str()
            .or_else(|| strings["en"][key].as_str())
            .unwrap_or("")
            .to_string()
    };
    let number = |value: f64| {
        let text = format!("{value:.1}");
        if language == "en" {
            text
        } else {
            text.replace('.', ",")
        }
    };
    let mut lines = Vec::new();
    if !options.top_only {
        lines.extend([
            label("app_name").to_uppercase(),
            format!("{}: {}", label("total_games"), games.len()),
            format!(
                "{}: {} / 10",
                label("average_rating"),
                average(games).map(number).unwrap_or("—".into())
            ),
        ]);
        if games.is_empty() {
            lines.push(label("empty_collection"));
        }
    }
    for (index, game) in games.iter().enumerate() {
        if !options.top_only {
            lines.push(String::new());
        }
        lines.push(format!(
            "{}. {} — {} / 10",
            index + 1,
            game.title,
            number(game.rating)
        ));
        if options.top_only {
            continue;
        }
        lines.push(format!(
            "{}: {}",
            label("status"),
            label(game.status.label())
        ));
        lines.push(format!("{}: {}", label("completions"), game.completions));
        let mut field = |key: &str, value: String| {
            if !value.trim().is_empty() {
                lines.push(format!("{}: {value}", label(key)));
            }
        };
        if options.include_metadata {
            field("series", game.series.clone().unwrap_or_default());
            field("universe", game.universe.clone().unwrap_or_default());
            field(
                "release_year",
                game.release_year
                    .map(|year| year.to_string())
                    .unwrap_or_default(),
            );
            for (key, values) in [
                ("genres", &game.genres),
                ("tags", &game.tags),
                ("developers", &game.developers),
                ("platforms", &game.platforms),
            ] {
                field(key, values.join(", "));
            }
        }
        if options.include_history {
            field("completion_dates", game.completion_dates.join(", "));
            field(
                "playtime",
                game.playtime_hours
                    .map(|hours| format!("{hours} {}", label("hours")))
                    .unwrap_or_default(),
            );
        }
        if options.include_opinion {
            field("opinion", game.opinion.clone().unwrap_or_default());
        }
        if options.include_cover {
            field("cover_link", game.cover_uri.clone().unwrap_or_default());
        }
    }
    Ok(lines.join("\n"))
}
