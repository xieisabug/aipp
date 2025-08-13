use std::error::Error as StdError;

use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    DatabaseError(String),

    #[error("IO错误: {0}")]
    IoError(String),

    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("未找到模型")]
    NoModelFound,

    #[error("大模型提供商错误: {0}")]
    ProviderError(String),

    #[error("消息通信错误: {0}")]
    WindowEmitError(String),

    #[error("未知错误: {0}")]
    UnknownError(String),

    #[error("运行代码错误: {0}")]
    RunCodeError(String),

    #[error("未进行配置: {0}")]
    NoConfigError(String),

    #[error("Anyhow错误: {0}")]
    Anyhow(String),

    #[error("对话不存在: {0}")]
    ConversationNotFound(i64),

    #[error("消息数量不足以生成标题")]
    InsufficientMessages,

    #[error("内部错误: {0}")]
    InternalError(String),
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::IoError(err.to_string())
    }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(err: std::num::ParseIntError) -> Self {
        AppError::ParseError(err.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(err: tauri::Error) -> Self {
        AppError::WindowEmitError(err.to_string())
    }
}

impl From<tauri_plugin_opener::Error> for AppError {
    fn from(err: tauri_plugin_opener::Error) -> Self {
        AppError::IoError(err.to_string())
    }
}

impl From<Box<dyn StdError>> for AppError {
    fn from(err: Box<dyn StdError>) -> Self {
        AppError::UnknownError(err.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Anyhow(err.to_string())
    }
}
