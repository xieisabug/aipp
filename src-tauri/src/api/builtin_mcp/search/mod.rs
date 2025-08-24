pub mod browser;
pub mod engines;
pub mod engine_manager;
pub mod fetcher;
pub mod handler;

pub use handler::SearchHandler;
pub use engine_manager::{SearchEngine, SearchEngineManager};