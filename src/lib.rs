pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;

#[cfg(feature = "desktop")]
pub mod presentation;

// Stable facade for consumers and the existing test suite. Internal dependencies
// still point inward through the explicit layer modules above.
pub use application::{decode_import, export, ExportOptions, MAX_IMPORT_BYTES, STRINGS};
pub use domain::{average, Appearance, Game, Query, Sort, Status, SyncSettings};
pub use error::AppResult as Result;
pub use infrastructure::SqliteStore as Store;
