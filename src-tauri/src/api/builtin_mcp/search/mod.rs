pub mod browser;
pub mod engines;
pub mod engine_manager;
pub mod fetcher;
pub mod handler;
pub mod types;

pub use handler::SearchHandler;
pub use engine_manager::{SearchEngine, SearchEngineManager};
pub use types::{SearchResultType, SearchItem, SearchResults, SearchRequest, SearchResponse};