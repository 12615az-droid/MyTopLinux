use super::{Game, Status};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sort {
    #[default]
    RatingDesc,
    RatingAsc,
    Title,
    Newest,
    ReleaseYear,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Query {
    pub search: String,
    pub status: Option<Status>,
    pub genre: Option<String>,
    pub minimum_rating: f64,
    pub sort: Sort,
}

impl Query {
    pub fn apply(&self, games: &[Game]) -> Vec<Game> {
        let words: Vec<_> = self
            .search
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        let mut result: Vec<_> = games
            .iter()
            .filter(|game| {
                let mut fields = vec![
                    game.title.as_str(),
                    game.series.as_deref().unwrap_or(""),
                    game.universe.as_deref().unwrap_or(""),
                ];
                fields.extend(
                    game.genres
                        .iter()
                        .chain(&game.tags)
                        .chain(&game.developers)
                        .chain(&game.platforms)
                        .map(String::as_str),
                );
                let searchable = fields.join(" ").to_lowercase();
                words.iter().all(|word| searchable.contains(word))
                    && self.status.is_none_or(|status| status == game.status)
                    && self.genre.as_ref().is_none_or(|genre| {
                        game.genres
                            .iter()
                            .any(|value| value.eq_ignore_ascii_case(genre))
                    })
                    && game.rating >= self.minimum_rating
            })
            .cloned()
            .collect();

        result.sort_by(|left, right| {
            let order = match self.sort {
                Sort::RatingDesc => right.rating.total_cmp(&left.rating),
                Sort::RatingAsc => left.rating.total_cmp(&right.rating),
                Sort::Title => Ordering::Equal,
                Sort::Newest => right.id.cmp(&left.id),
                Sort::ReleaseYear => right
                    .release_year
                    .unwrap_or(i32::MIN)
                    .cmp(&left.release_year.unwrap_or(i32::MIN)),
            };
            order
                .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
                .then_with(|| left.id.cmp(&right.id))
        });
        result
    }
}

pub fn average(games: &[Game]) -> Option<f64> {
    (!games.is_empty())
        .then(|| games.iter().map(|game| game.rating).sum::<f64>() / games.len() as f64)
}
