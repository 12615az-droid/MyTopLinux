use crate::{domain::Game, error::AppResult};
use serde::{de::DeserializeOwned, Serialize};

pub trait GameRepository {
    fn games(&self) -> AppResult<Vec<Game>>;
    fn save(&mut self, game: Game) -> AppResult<Game>;
    fn delete(&mut self, id: i64) -> AppResult<()>;
    fn import_games(&mut self, games: &[Game]) -> AppResult<usize>;
}

pub trait SettingsRepository {
    fn setting<T: DeserializeOwned + Default>(&self, key: &str) -> AppResult<T>;
    fn set_setting<T: Serialize>(&mut self, key: &str, value: &T) -> AppResult<()>;
}
