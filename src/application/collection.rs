use super::{decode_import, GameRepository};
use crate::{
    domain::{average, Game, Query},
    error::AppResult,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub games: Vec<Game>,
    pub total: usize,
    pub average: Option<f64>,
    pub genres: Vec<String>,
}

pub struct CollectionService<'a, R> {
    repository: &'a mut R,
}

impl<'a, R: GameRepository> CollectionService<'a, R> {
    pub fn new(repository: &'a mut R) -> Self {
        Self { repository }
    }

    pub fn collection(&self, query: &Query) -> AppResult<Collection> {
        let all = self.repository.games()?;
        let games = query.apply(&all);
        let mut genres: Vec<_> = all.iter().flat_map(|game| game.genres.clone()).collect();
        genres.sort_by_key(|genre| genre.to_lowercase());
        genres.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        Ok(Collection {
            total: all.len(),
            average: average(&games),
            games,
            genres,
        })
    }

    pub fn save(&mut self, game: Game) -> AppResult<Game> {
        self.repository.save(game.normalized()?)
    }
    pub fn delete(&mut self, id: i64) -> AppResult<()> {
        self.repository.delete(id)
    }
    pub fn import(&mut self, text: &str) -> AppResult<usize> {
        let games = decode_import(text)?;
        self.repository.import_games(&games)
    }
}
