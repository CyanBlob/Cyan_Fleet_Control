#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::AppState;
pub use app::AppData;
pub use app::api::spacetraders;
pub use app::api::message_handler;
