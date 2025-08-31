use chrono::prelude::*;
use rusqlite::{Row, Error as SqliteError};

/// 统一的DateTime类型转换辅助函数
/// 用于处理SQLite中DATETIME字段的不同存储格式（字符串或时间戳）
/// 
/// 这个函数能够处理以下SQLite中DATETIME的存储格式：
/// 1. ISO 8601字符串格式 (如 "2024-01-01T10:00:00Z")
/// 2. Unix时间戳整数格式 - 支持秒级和毫秒级 (如 1704106800 或 1704106800000)
/// 3. NULL值
pub fn get_datetime_from_row(row: &Row, index: usize) -> Result<Option<DateTime<Utc>>, SqliteError> {
    match row.get::<_, Option<String>>(index) {
        Ok(Some(time_str)) => {
            // 如果是字符串格式，尝试解析
            Ok(time_str.parse().ok())
        }
        Ok(None) => Ok(None),
        Err(_) => {
            // 如果不是字符串，尝试作为时间戳处理
            match row.get::<_, Option<i64>>(index) {
                Ok(Some(timestamp)) => {
                    // 智能判断时间戳格式：毫秒级 vs 秒级
                    let datetime = if timestamp > 1_000_000_000_000 {
                        // 毫秒级时间戳 (> 1万亿，即 2001年以后的毫秒时间戳)
                        DateTime::from_timestamp_millis(timestamp)
                    } else {
                        // 秒级时间戳
                        DateTime::from_timestamp(timestamp, 0)
                    };
                    Ok(datetime)
                }
                Ok(None) => Ok(None),
                Err(_) => Ok(None),
            }
        }
    }
}

/// 统一的非空DateTime类型转换辅助函数
/// 用于处理必须有值的DateTime字段
/// 
/// 这个函数与 get_datetime_from_row 类似，但是对于必须有值的字段，
/// 如果解析失败会返回适当的错误信息
pub fn get_required_datetime_from_row(row: &Row, index: usize, field_name: &str) -> Result<DateTime<Utc>, SqliteError> {
    match row.get::<_, String>(index) {
        Ok(time_str) => {
            // 如果是字符串格式，尝试解析
            time_str.parse().map_err(|_| SqliteError::InvalidColumnType(
                index, field_name.to_string(), rusqlite::types::Type::Text
            ))
        }
        Err(_) => {
            // 如果不是字符串，尝试作为时间戳处理
            let timestamp: i64 = row.get(index)?;
            
            // 智能判断时间戳格式：毫秒级 vs 秒级
            let datetime = if timestamp > 1_000_000_000_000 {
                // 毫秒级时间戳 (> 1万亿，即 2001年以后的毫秒时间戳)
                DateTime::from_timestamp_millis(timestamp)
            } else {
                // 秒级时间戳
                DateTime::from_timestamp(timestamp, 0)
            };
            
            datetime.ok_or_else(|| SqliteError::InvalidColumnType(
                index, field_name.to_string(), rusqlite::types::Type::Integer
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{Connection, Result};
    use chrono::Utc;

    #[test]
    fn test_datetime_conversion_string() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        conn.execute(
            "CREATE TABLE test (id INTEGER, datetime_field TEXT)",
            [],
        )?;
        
        let test_time = "2024-01-01T10:00:00Z";
        conn.execute(
            "INSERT INTO test (id, datetime_field) VALUES (1, ?)",
            [test_time],
        )?;

        let result = conn.query_row(
            "SELECT datetime_field FROM test WHERE id = 1",
            [],
            |row| get_required_datetime_from_row(row, 0, "datetime_field")
        )?;

        assert_eq!(result.to_rfc3339(), test_time);
        Ok(())
    }

    #[test]
    fn test_datetime_conversion_timestamp() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        conn.execute(
            "CREATE TABLE test (id INTEGER, datetime_field INTEGER)",
            [],
        )?;
        
        let test_timestamp = 1704106800i64; // 2024-01-01T10:00:00Z
        conn.execute(
            "INSERT INTO test (id, datetime_field) VALUES (1, ?)",
            [test_timestamp],
        )?;

        let result = conn.query_row(
            "SELECT datetime_field FROM test WHERE id = 1",
            [],
            |row| get_required_datetime_from_row(row, 0, "datetime_field")
        )?;

        let expected = DateTime::from_timestamp(test_timestamp, 0).unwrap();
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_datetime_conversion_millisecond_timestamp() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        conn.execute(
            "CREATE TABLE test (id INTEGER, datetime_field INTEGER)",
            [],
        )?;
        
        // 测试毫秒级时间戳 (类似你提供的 1756646536000)
        let test_timestamp = 1756646536000i64; // 2025-09-01 10:35:36 UTC
        conn.execute(
            "INSERT INTO test (id, datetime_field) VALUES (1, ?)",
            [test_timestamp],
        )?;

        let result = conn.query_row(
            "SELECT datetime_field FROM test WHERE id = 1",
            [],
            |row| get_required_datetime_from_row(row, 0, "datetime_field")
        )?;

        // 验证转换后的时间是否正确 (毫秒时间戳应该转换为对应的日期时间)
        let expected = DateTime::from_timestamp_millis(test_timestamp).unwrap();
        assert_eq!(result, expected);
        
        // 确保不是异常的57635年
        assert!(result.year() > 2020 && result.year() < 2100);
        Ok(())
    }

    #[test] 
    fn test_optional_datetime_conversion_null() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        conn.execute(
            "CREATE TABLE test (id INTEGER, datetime_field TEXT)",
            [],
        )?;
        
        conn.execute(
            "INSERT INTO test (id, datetime_field) VALUES (1, NULL)",
            [],
        )?;

        let result = conn.query_row(
            "SELECT datetime_field FROM test WHERE id = 1",
            [],
            |row| get_datetime_from_row(row, 0)
        )?;

        assert_eq!(result, None);
        Ok(())
    }
}