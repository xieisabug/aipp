use anyhow::{anyhow, Result};
use base64::encode;
use serde::Serialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::{
    db::conversation_db::{ConversationDatabase, MessageAttachment},
    errors::AppError,
};

#[derive(Serialize)]
pub struct AttachmentResult {
    attachment_id: i64,
}

#[tauri::command]
pub async fn add_attachment(
    app_handle: tauri::AppHandle,
    file_url: String,
) -> Result<AttachmentResult, AppError> {
    // 1. 解析文件路径
    let file_path = Path::new(&file_url).to_path_buf();

    // 2. 检查文件是否存在
    if !file_path.exists() {
        return Err(AppError::Anyhow(anyhow!("File not found").to_string()));
    }

    // 3. 解析文件类型
    let file_extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| AppError::Anyhow(anyhow!("Invalid file URL").to_string()))?;
    let file_type = file_extension.to_lowercase();

    let file_type_map = [
        ("jpg", "image/jpeg"),
        ("jpeg", "image/jpeg"),
        ("png", "image/png"),
        ("gif", "image/gif"),
        ("webp", "image/webp"),
    ];

    let supported_types = file_type_map
        .iter()
        .map(|(ext, _)| *ext)
        .collect::<Vec<_>>();
    if !supported_types.contains(&file_type.as_str()) {
        return Err(AppError::Anyhow(
            anyhow!("Unsupported file type").to_string(),
        ));
    }

    let db = ConversationDatabase::new(&app_handle).map_err(AppError::from)?;

    // 4. 使用不同类型的文件读取方式来进行读取
    let reader = match file_type.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" => {
            // 使用 BufReader 读取图片文件
            let base64_str =
                read_image_as_base64(file_path.to_str().unwrap()).map_err(AppError::from)?;
            match file_type.as_str() {
                "jpg" | "jpeg" => "data:image/jpeg;base64,".to_string() + &base64_str,
                "png" => "data:image/png;base64,".to_string() + &base64_str,
                "gif" => "data:image/gif;base64,".to_string() + &base64_str,
                "webp" => "data:image/webp;base64,".to_string() + &base64_str,
                _ => {
                    return Err(AppError::Anyhow(
                        anyhow!("Unsupported file type").to_string(),
                    ))
                }
            }
        }
        _ => {
            return Err(AppError::Anyhow(
                anyhow!("Unsupported file type").to_string(),
            ))
        }
    };

    // 5. 保存到数据库
    // todo: 添加数据库配置和 CRUD 操作
    let attachment_id = match file_type.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" => {
            // 使用 BufReader 读取图片文件
            let message_attachment = MessageAttachment::create(
                &db.conn,
                -1,
                crate::db::conversation_db::AttachmentType::Image,
                Some(file_url),
                Some(reader),
                false,
                Some(0),
            );
            match message_attachment {
                Ok(t) => t.id,
                Err(e) => return Err(AppError::from(e)),
            }
        }
        _ => {
            return Err(AppError::Anyhow(
                anyhow!("Unsupported file type").to_string(),
            ))
        }
    };

    // 6. 返回到前端 attachment_id，等待之后的 message 创建和更新
    Ok(AttachmentResult { attachment_id })
}

#[tauri::command]
pub async fn add_attachment_base64(
    app_handle: tauri::AppHandle,
    file_content: String,
    file_name: String,
) -> Result<AttachmentResult, AppError> {
    println!("add_attachment_base64 file_name: {}", file_name);
    let db = ConversationDatabase::new(&app_handle).map_err(AppError::from)?;

    let message_attachment = MessageAttachment::create(
        &db.conn,
        -1,
        crate::db::conversation_db::AttachmentType::Image,
        Some(file_name),
        Some(file_content),
        false,
        Some(0),
    );
    let attachment_id = match message_attachment {
        Ok(t) => t.id,
        Err(e) => return Err(AppError::from(e)),
    };
    Ok(AttachmentResult { attachment_id })
}

fn read_image_as_base64(file_path: &str) -> Result<String> {
    // 打开文件
    let mut file = File::open(file_path)?;

    // 读取文件内容到字节向量
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let base64_string = encode(&buffer);
    Ok(base64_string)
}
