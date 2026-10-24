use crate::{
    application::{GameRepository, SettingsRepository},
    domain::Game,
    error::AppResult,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

pub struct SqliteStore {
    connection: Connection,
}

impl SqliteStore {
    pub fn open(path: &Path) -> AppResult<Self> {
        let connection = Connection::open(path).map_err(|error| error.to_string())?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|error| error.to_string())?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS games (id INTEGER PRIMARY KEY AUTOINCREMENT, record TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        ).map_err(|error| error.to_string())?;
        Ok(Self { connection })
    }

    // Convenience methods keep the public API small for callers and tests while
    // the application layer depends only on repository traits.
    pub fn games(&self) -> AppResult<Vec<Game>> {
        GameRepository::games(self)
    }
    pub fn save(&mut self, game: Game) -> AppResult<Game> {
        GameRepository::save(self, game)
    }
    pub fn delete(&mut self, id: i64) -> AppResult<()> {
        GameRepository::delete(self, id)
    }
    pub fn import(&mut self, text: &str) -> AppResult<usize> {
        let games = crate::application::decode_import(text)?;
        self.import_games(&games)
    }
    pub fn setting<T: DeserializeOwned + Default>(&self, key: &str) -> AppResult<T> {
        SettingsRepository::setting(self, key)
    }
    pub fn set_setting<T: Serialize>(&mut self, key: &str, value: &T) -> AppResult<()> {
        SettingsRepository::set_setting(self, key, value)
    }
}

impl GameRepository for SqliteStore {
    fn games(&self) -> AppResult<Vec<Game>> {
        let mut statement = self
            .connection
            .prepare("SELECT id, record FROM games ORDER BY id")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| error.to_string())?;
        rows.map(|row| {
            let (id, record) = row.map_err(|error| error.to_string())?;
            let mut game: Game =
                serde_json::from_str(&record).map_err(|error| error.to_string())?;
            game.id = id;
            Ok(game)
        })
        .collect()
    }

    fn save(&mut self, mut game: Game) -> AppResult<Game> {
        game = game.normalized()?;
        let record = serde_json::to_string(&game).map_err(|error| error.to_string())?;
        if game.id == 0 {
            self.connection
                .execute("INSERT INTO games(record) VALUES (?)", [&record])
                .map_err(|error| error.to_string())?;
            game.id = self.connection.last_insert_rowid();
        } else if self
            .connection
            .execute(
                "UPDATE games SET record=? WHERE id=?",
                params![record, game.id],
            )
            .map_err(|error| error.to_string())?
            == 0
        {
            return Err("game_not_found".into());
        }
        Ok(game)
    }

    fn delete(&mut self, id: i64) -> AppResult<()> {
        if self
            .connection
            .execute("DELETE FROM games WHERE id=?", [id])
            .map_err(|error| error.to_string())?
            == 0
        {
            return Err("already_deleted".into());
        }
        Ok(())
    }

    fn import_games(&mut self, games: &[Game]) -> AppResult<usize> {
        let transaction = self
            .connection
            .transaction()
            .map_err(|error| error.to_string())?;
        for game in games {
            transaction
                .execute(
                    "INSERT INTO games(record) VALUES (?)",
                    [serde_json::to_string(game).map_err(|error| error.to_string())?],
                )
                .map_err(|error| error.to_string())?;
        }
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(games.len())
    }
}

impl SettingsRepository for SqliteStore {
    fn setting<T: DeserializeOwned + Default>(&self, key: &str) -> AppResult<T> {
        let value: Option<String> = self
            .connection
            .query_row("SELECT value FROM settings WHERE key=?", [key], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|error| error.to_string())?;
        value
            .map(|text| serde_json::from_str(&text).map_err(|error| error.to_string()))
            .unwrap_or_else(|| Ok(T::default()))
    }

    fn set_setting<T: Serialize>(&mut self, key: &str, value: &T) -> AppResult<()> {
        self.connection.execute(
            "INSERT INTO settings(key,value) VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, serde_json::to_string(value).map_err(|error| error.to_string())?],
        ).map_err(|error| error.to_string())?;
        Ok(())
    }
}
