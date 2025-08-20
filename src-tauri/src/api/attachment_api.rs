use crate::db::conversation_db::{AttachmentType, Repository};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use mime_guess::from_path;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tauri_plugin_opener::OpenerExt;

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
    file_url: Option<String>,
    file_content: Option<String>,
    file_name: Option<String>,
    attachment_type: Option<i64>,
) -> Result<AttachmentResult, AppError> {
    println!("add_attachment file_url: {:?} file_name: {:?}", file_url, file_name);
    // 如果有 URL，使用 add_attachment_by_url
    if let Some(url) = file_url {
        return add_attachment_by_url(app_handle, url).await;
    }

    // 如果有 content 和其他必要参数，使用 add_attachment_content
    if let (Some(content), Some(name), Some(att_type)) = (file_content, file_name, attachment_type)
    {
        return add_attachment_content(app_handle, content, name, att_type).await;
    }

    // 如果都没有提供有效参数，返回错误
    Err(AppError::Anyhow(
        "无效的参数：需要提供文件URL或者（文件内容、文件名、附件类型）".to_string(),
    ))
}

pub async fn add_attachment_by_url(
    app_handle: tauri::AppHandle,
    file_url: String,
) -> Result<AttachmentResult, AppError> {
    // 1. 解析文件路径
    let file_path = Path::new(&file_url).to_path_buf();

    // 2. 检查文件是否存在
    if !file_path.exists() {
        return Err(AppError::Anyhow(anyhow!("找不到对应的文件").to_string()));
    }

    // 3. 解析文件类型
    let file_type = from_path(&file_path).first_or_octet_stream().to_string();
    let mut file_type_classify = String::new();

    println!("检测到的文件类型: {}", file_type);

    if file_type.starts_with("text/") {
        file_type_classify = "text".to_string();
    } else if file_type.starts_with("image/") {
        file_type_classify = "image".to_string();
    }
    println!("文件类型大类: {}", file_type_classify);

    let db = ConversationDatabase::new(&app_handle).map_err(AppError::from)?;

    // 4. 使用不同类型的文件读取方式来进行读取
    let reader = match file_type_classify.as_str() {
        "image" => {
            // 使用 BufReader 读取图片文件
            let base64_str =
                read_image_as_base64(file_path.to_str().unwrap()).map_err(AppError::from)?;
            match file_type.as_str() {
                "image/jpeg" => "data:image/jpeg;base64,".to_string() + &base64_str,
                "image/png" => "data:image/png;base64,".to_string() + &base64_str,
                "image/gif" => "data:image/gif;base64,".to_string() + &base64_str,
                "image/webp" => "data:image/webp;base64,".to_string() + &base64_str,
                _ => return Err(AppError::Anyhow(anyhow!("Unsupported file type").to_string())),
            }
        }
        "text" => {
            // 读取文本文件
            let mut file = File::open(file_path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            content
        }
        _ => return Err(AppError::Anyhow(anyhow!("Unsupported file type").to_string())),
    };

    let mut hasher = Sha256::new();
    hasher.update(reader.as_bytes());
    let hash_str = hex::encode(hasher.finalize());

    println!("file hash: {}", hash_str);

    // 去数据库根据sha256的数据查看是否有相同的attachment
    let option_attachment =
        db.attachment_repo().unwrap().read_by_attachment_hash(hash_str.as_str())?;
    match option_attachment {
        Some(attachment) => {
            println!("add_attachment 找到相同的sha256: {}", attachment.id);
            return Ok(AttachmentResult { attachment_id: attachment.id });
        }
        None => {
            // 5. 保存到数据库
            // todo: 添加数据库配置和 CRUD 操作
            let attachment_id = match file_type_classify.as_str() {
                "image" => {
                    // 使用 BufReader 读取图片文件
                    let message_attachment =
                        db.attachment_repo().unwrap().create(&MessageAttachment {
                            id: 0,
                            message_id: -1,
                            attachment_type: AttachmentType::Image,
                            attachment_url: Some(file_url),
                            attachment_content: Some(reader),
                            attachment_hash: Some(hash_str),
                            use_vector: false,
                            token_count: Some(0),
                        })?;
                    message_attachment.id
                }
                "text" => {
                    // 使用 BufReader 读取图片文件
                    let message_attachment =
                        db.attachment_repo().unwrap().create(&MessageAttachment {
                            id: 0,
                            message_id: -1,
                            attachment_type: AttachmentType::Text,
                            attachment_url: Some(file_url),
                            attachment_content: Some(reader),
                            attachment_hash: Some(hash_str),
                            use_vector: false,
                            token_count: Some(0),
                        })?;
                    message_attachment.id
                }
                _ => return Err(AppError::Anyhow(anyhow!("Unsupported file type").to_string())),
            };

            // 6. 返回到前端 attachment_id，等待之后的 message 创建和更新
            Ok(AttachmentResult { attachment_id })
        }
    }
}

pub async fn add_attachment_content(
    app_handle: tauri::AppHandle,
    file_content: String,
    file_name: String,
    attachment_type: i64,
) -> Result<AttachmentResult, AppError> {
    println!("add_attachment_content file_name: {}", file_name);
    let db = ConversationDatabase::new(&app_handle).map_err(AppError::from)?;

    let mut hasher = Sha256::new();
    hasher.update(file_content.clone());
    let hash_str = hex::encode(hasher.finalize());

    println!("file hash: {}", hash_str);

    // 去数据库根据sha256的数据查看是否有相同的attachment
    let option_attachment =
        db.attachment_repo().unwrap().read_by_attachment_hash(hash_str.as_str())?;

    match option_attachment {
        Some(attachment) => {
            println!("add_attachment_content 找到相同的sha256: {}", attachment.id);
            return Ok(AttachmentResult { attachment_id: attachment.id });
        }
        None => {
            let message_attachment = db.attachment_repo().unwrap().create(&MessageAttachment {
                id: 0,
                message_id: -1,
                attachment_type: AttachmentType::try_from(attachment_type).unwrap(),
                attachment_url: Some(file_name),
                attachment_content: Some(file_content),
                attachment_hash: Some(hash_str),
                use_vector: false,
                token_count: Some(0),
            });
            let attachment_id = match message_attachment {
                Ok(t) => t.id,
                Err(e) => return Err(AppError::from(e)),
            };
            Ok(AttachmentResult { attachment_id })
        }
    }
}

#[tauri::command]
pub async fn open_attachment_with_default_app(
    id: i64,
    app_handle: tauri::AppHandle,
) -> Result<(), AppError> {
    let db = ConversationDatabase::new(&app_handle).map_err(AppError::from)?;
    let attachment = db.attachment_repo().unwrap().read(id)?;

    let file_path =
        Path::new("attachments").join(attachment.unwrap().attachment_url.as_ref().unwrap());
    let file_path = file_path.to_str().unwrap();
    println!("file_path: {}", file_path);

    let opener = app_handle.opener();
    opener.open_path(file_path, None::<&str>)?;
    Ok(())
}

fn read_image_as_base64(file_path: &str) -> Result<String> {
    // 打开文件
    let mut file = File::open(file_path)?;

    // 读取文件内容到字节向量
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let base64_string = STANDARD.encode(&buffer);
    Ok(base64_string)
}
