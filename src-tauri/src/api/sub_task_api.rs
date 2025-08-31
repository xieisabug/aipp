use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::{
    db::{
        conversation_db::{ConversationDatabase, Repository as ConversationRepository},
        sub_task_db::{
            Repository, SubTaskDatabase, SubTaskDefinition, SubTaskExecution,
            SubTaskExecutionSummary,
        },
    },
};

// 事件定义
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubTaskStatusUpdateEvent {
    pub execution_id: i64,
    pub task_code: String,
    pub task_name: String,
    pub parent_conversation_id: i64,
    pub parent_message_id: Option<i64>,
    pub status: String,
    pub result_content: Option<String>,
    pub error_message: Option<String>,
    pub token_count: Option<i32>,
    pub started_time: Option<chrono::DateTime<chrono::Utc>>,
    pub finished_time: Option<chrono::DateTime<chrono::Utc>>,
}

// 参数覆盖结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubTaskExecutionParams {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<i32>,
    pub custom_model_id: Option<i64>,
}

// 创建子任务请求
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateSubTaskRequest {
    pub task_code: String,
    pub task_prompt: String,
    pub parent_conversation_id: i64,
    pub parent_message_id: Option<i64>,
    pub source_id: i64,
    pub ai_params: Option<SubTaskExecutionParams>,
}

// 鉴权辅助函数
async fn validate_source_permission(
    app_handle: &tauri::AppHandle,
    source_id: i64,
    plugin_source: &str,
    _operation: &str, // 'read' | 'write' | 'delete'
) -> Result<bool, String> {
    match plugin_source {
        "mcp" => {
            // 验证 MCP 服务器权限
            let mcp_db = crate::mcp::mcp_db::MCPDatabase::new(app_handle)
                .map_err(|e| format!("创建MCP数据库连接失败: {}", e))?;
            let server = mcp_db.get_mcp_server(source_id)
                .map_err(|e| format!("获取MCP服务器失败: {}", e))?;
            Ok(server.is_enabled)
        }
        "plugin" => {
            // 验证插件权限 (目前先返回 true，后续可以扩展)
            Ok(true)
        }
        _ => Ok(false),
    }
}

// 发送状态更新事件
async fn emit_sub_task_status_update(
    app_handle: &tauri::AppHandle,
    execution: &SubTaskExecution,
) {
    let event = SubTaskStatusUpdateEvent {
        execution_id: execution.id,
        task_code: execution.task_code.clone(),
        task_name: execution.task_name.clone(),
        parent_conversation_id: execution.parent_conversation_id,
        parent_message_id: execution.parent_message_id,
        status: execution.status.clone(),
        result_content: execution.result_content.clone(),
        error_message: execution.error_message.clone(),
        token_count: Some(execution.token_count),
        started_time: execution.started_time,
        finished_time: execution.finished_time,
    };

    let _ = app_handle.emit(
        &format!("sub_task_update_{}", execution.parent_conversation_id),
        event,
    );
}

// 任务定义管理 API

#[tauri::command]
pub async fn register_sub_task_definition(
    app_handle: tauri::AppHandle,
    name: String,
    code: String,
    description: String,
    system_prompt: String,
    plugin_source: String, // 'mcp' | 'plugin'
    source_id: i64,
) -> Result<i64, String> {
    // 鉴权检查
    if !validate_source_permission(&app_handle, source_id, &plugin_source, "write").await? {
        return Err("没有权限注册此任务定义".to_string());
    }

    let db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let repo = db.definition_repo().map_err(|e| e.to_string())?;

    // 检查 code 是否已存在
    if repo.find_by_code(&code).map_err(|e| e.to_string())?.is_some() {
        return Err(format!("任务代码 '{}' 已存在", code));
    }

    let definition = SubTaskDefinition {
        id: 0,
        name,
        code,
        description,
        system_prompt,
        plugin_source,
        source_id,
        is_enabled: true,
        created_time: Utc::now(),
        updated_time: Utc::now(),
    };

    let created = repo.create(&definition).map_err(|e| e.to_string())?;
    Ok(created.id)
}

#[tauri::command]
pub async fn list_sub_task_definitions(
    app_handle: tauri::AppHandle,
    plugin_source: Option<String>, // 过滤条件
    source_id: Option<i64>,        // 过滤条件
    is_enabled: Option<bool>,      // 过滤条件
) -> Result<Vec<SubTaskDefinition>, String> {
    let db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let repo = db.definition_repo().map_err(|e| e.to_string())?;

    let definitions = repo
        .list_by_source(
            plugin_source.as_deref(),
            source_id,
            is_enabled,
        )
        .map_err(|e| e.to_string())?;

    // 鉴权过滤：只返回有权限的任务定义
    let mut filtered_definitions = Vec::new();
    for def in definitions {
        if validate_source_permission(&app_handle, def.source_id, &def.plugin_source, "read")
            .await?
        {
            filtered_definitions.push(def);
        }
    }

    Ok(filtered_definitions)
}

#[tauri::command]
pub async fn get_sub_task_definition(
    app_handle: tauri::AppHandle,
    code: String,
    source_id: i64, // 鉴权参数
) -> Result<Option<SubTaskDefinition>, String> {
    let db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let repo = db.definition_repo().map_err(|e| e.to_string())?;

    if let Some(definition) = repo.find_by_code(&code).map_err(|e| e.to_string())? {
        // 鉴权检查
        if definition.source_id != source_id {
            return Err("没有权限访问此任务定义".to_string());
        }

        if validate_source_permission(
            &app_handle,
            definition.source_id,
            &definition.plugin_source,
            "read",
        )
        .await?
        {
            Ok(Some(definition))
        } else {
            Err("没有权限访问此任务定义".to_string())
        }
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn update_sub_task_definition(
    app_handle: tauri::AppHandle,
    id: i64,
    name: Option<String>,
    description: Option<String>,
    system_prompt: Option<String>,
    is_enabled: Option<bool>,
    source_id: i64, // 鉴权参数
) -> Result<(), String> {
    let db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let repo = db.definition_repo().map_err(|e| e.to_string())?;

    // 获取现有定义并检查权限
    if let Some(mut definition) = repo.read(id).map_err(|e| e.to_string())? {
        if definition.source_id != source_id {
            return Err("没有权限更新此任务定义".to_string());
        }

        if !validate_source_permission(
            &app_handle,
            definition.source_id,
            &definition.plugin_source,
            "write",
        )
        .await?
        {
            return Err("没有权限更新此任务定义".to_string());
        }

        // 更新字段
        if let Some(n) = name {
            definition.name = n;
        }
        if let Some(d) = description {
            definition.description = d;
        }
        if let Some(s) = system_prompt {
            definition.system_prompt = s;
        }
        if let Some(e) = is_enabled {
            definition.is_enabled = e;
        }

        definition.updated_time = Utc::now();

        repo.update(&definition).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("任务定义不存在".to_string())
    }
}

#[tauri::command]
pub async fn delete_sub_task_definition(
    app_handle: tauri::AppHandle,
    id: i64,
    source_id: i64, // 鉴权参数
) -> Result<(), String> {
    let db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let repo = db.definition_repo().map_err(|e| e.to_string())?;

    // 获取现有定义并检查权限
    if let Some(definition) = repo.read(id).map_err(|e| e.to_string())? {
        if definition.source_id != source_id {
            return Err("没有权限删除此任务定义".to_string());
        }

        if !validate_source_permission(
            &app_handle,
            definition.source_id,
            &definition.plugin_source,
            "delete",
        )
        .await?
        {
            return Err("没有权限删除此任务定义".to_string());
        }

        repo.delete(id).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("任务定义不存在".to_string())
    }
}

// 任务执行管理 API

#[tauri::command]
pub async fn create_sub_task_execution(
    app_handle: tauri::AppHandle,
    request: CreateSubTaskRequest,
) -> Result<i64, String> {
    // 获取任务定义并验证权限
    let sub_task_db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let def_repo = sub_task_db.definition_repo().map_err(|e| e.to_string())?;

    let task_definition = def_repo
        .find_by_code(&request.task_code)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("任务定义 '{}' 不存在", request.task_code))?;

    // 鉴权检查
    if task_definition.source_id != request.source_id {
        return Err("没有权限执行此任务".to_string());
    }

    if !validate_source_permission(
        &app_handle,
        task_definition.source_id,
        &task_definition.plugin_source,
        "write",
    )
    .await?
    {
        return Err("没有权限执行此任务".to_string());
    }

    // 检查任务是否启用
    if !task_definition.is_enabled {
        return Err("任务定义已禁用".to_string());
    }

    // 验证父对话是否存在
    let conv_db = ConversationDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let conv_repo = conv_db.conversation_repo().map_err(|e| e.to_string())?;
    
    if conv_repo
        .read(request.parent_conversation_id)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        return Err("父对话不存在".to_string());
    }

    // 如果指定了父消息，验证其存在性
    if let Some(msg_id) = request.parent_message_id {
        let msg_repo = conv_db.message_repo().map_err(|e| e.to_string())?;
        if msg_repo.read(msg_id).map_err(|e| e.to_string())?.is_none() {
            return Err("父消息不存在".to_string());
        }
    }

    // 创建执行记录
    let exec_repo = sub_task_db.execution_repo().map_err(|e| e.to_string())?;
    let execution = SubTaskExecution {
        id: 0,
        task_definition_id: task_definition.id,
        task_code: request.task_code.clone(),
        task_name: task_definition.name.clone(),
        task_prompt: request.task_prompt.clone(),
        parent_conversation_id: request.parent_conversation_id,
        parent_message_id: request.parent_message_id,
        status: "pending".to_string(),
        result_content: None,
        error_message: None,
        llm_model_id: None,
        llm_model_name: None,
        token_count: 0,
        input_token_count: 0,
        output_token_count: 0,
        started_time: None,
        finished_time: None,
        created_time: Utc::now(),
    };

    let created_execution = exec_repo.create(&execution).map_err(|e| e.to_string())?;
    let execution_id = created_execution.id;

    // 异步执行任务
    let app_handle_clone = app_handle.clone();
    let task_def_clone = task_definition.clone();
    let task_prompt_clone = request.task_prompt.clone();
    let _ai_params_clone = request.ai_params.clone();

    tokio::spawn(async move {
        // 更新状态为 running
        let sub_task_db = SubTaskDatabase::new(&app_handle_clone).unwrap();
        let exec_repo = sub_task_db.execution_repo().unwrap();
        let started_time = Utc::now();
        
        let _ = exec_repo.update_status(execution_id, "running", Some(started_time));

        // 发送状态更新事件
        let mut updated_execution = exec_repo.read(execution_id).unwrap().unwrap();
        updated_execution.status = "running".to_string();
        updated_execution.started_time = Some(started_time);
        emit_sub_task_status_update(&app_handle_clone, &updated_execution).await;

        // 简化执行任务：暂时返回固定结果
        let result: Result<(String, Option<(i32, i32, i32)>), String> = Ok((
            format!("执行任务 '{}' 完成，输入: {}", task_def_clone.name, task_prompt_clone), 
            Some((100, 50, 50))
        ));

        // 更新执行结果
        let finished_time = Utc::now();
        match result {
            Ok((content, token_stats)) => {
                let _ = exec_repo.update_result(
                    execution_id,
                    "success",
                    Some(&content),
                    None,
                    token_stats,
                    Some(finished_time),
                );
            }
            Err(error) => {
                let _ = exec_repo.update_result(
                    execution_id,
                    "failed",
                    None,
                    Some(&error),
                    None,
                    Some(finished_time),
                );
            }
        }

        // 发送完成事件
        if let Ok(Some(final_execution)) = exec_repo.read(execution_id) {
            emit_sub_task_status_update(&app_handle_clone, &final_execution).await;
        }
    });

    Ok(execution_id)
}

#[tauri::command]
pub async fn list_sub_task_executions(
    app_handle: tauri::AppHandle,
    parent_conversation_id: i64,
    parent_message_id: Option<i64>,
    status: Option<String>,     // 过滤条件
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<Vec<SubTaskExecutionSummary>, String> {
    let db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let repo = db.execution_repo().map_err(|e| e.to_string())?;

    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);

    let executions = repo.list_by_conversation(
            parent_conversation_id,
            parent_message_id,
            status.as_deref(),
            page,
            page_size,
        )
        .map_err(|e| e.to_string())?;
    
    Ok(executions)
}

#[tauri::command]
pub async fn get_sub_task_execution_detail(
    app_handle: tauri::AppHandle,
    execution_id: i64,
    source_id: i64, // 鉴权参数
) -> Result<Option<SubTaskExecution>, String> {
    let db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let exec_repo = db.execution_repo().map_err(|e| e.to_string())?;
    let def_repo = db.definition_repo().map_err(|e| e.to_string())?;

    if let Some(execution) = exec_repo.read(execution_id).map_err(|e| e.to_string())? {
        // 获取任务定义进行鉴权检查
        if let Some(definition) = def_repo
            .read(execution.task_definition_id)
            .map_err(|e| e.to_string())?
        {
            if definition.source_id != source_id {
                return Err("没有权限访问此任务执行详情".to_string());
            }

            if validate_source_permission(
                &app_handle,
                definition.source_id,
                &definition.plugin_source,
                "read",
            )
            .await?
            {
                Ok(Some(execution))
            } else {
                Err("没有权限访问此任务执行详情".to_string())
            }
        } else {
            Err("关联的任务定义不存在".to_string())
        }
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn cancel_sub_task_execution(
    app_handle: tauri::AppHandle,
    execution_id: i64,
    source_id: i64, // 鉴权参数
) -> Result<(), String> {
    let db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let exec_repo = db.execution_repo().map_err(|e| e.to_string())?;
    let def_repo = db.definition_repo().map_err(|e| e.to_string())?;

    if let Some(execution) = exec_repo.read(execution_id).map_err(|e| e.to_string())? {
        // 获取任务定义进行鉴权检查
        if let Some(definition) = def_repo
            .read(execution.task_definition_id)
            .map_err(|e| e.to_string())?
        {
            if definition.source_id != source_id {
                return Err("没有权限取消此任务执行".to_string());
            }

            if !validate_source_permission(
                &app_handle,
                definition.source_id,
                &definition.plugin_source,
                "write",
            )
            .await?
            {
                return Err("没有权限取消此任务执行".to_string());
            }

            // 只有 pending 或 running 状态的任务可以取消
            if execution.status != "pending" && execution.status != "running" {
                return Err(format!("任务状态为 '{}' 时无法取消", execution.status));
            }

            // 更新状态为 cancelled
            exec_repo
                .update_status(execution_id, "cancelled", None)
                .map_err(|e| e.to_string())?;

            // 发送状态更新事件
            if let Ok(Some(updated_execution)) = exec_repo.read(execution_id) {
                emit_sub_task_status_update(&app_handle, &updated_execution).await;
            }

            Ok(())
        } else {
            Err("关联的任务定义不存在".to_string())
        }
    } else {
        Err("任务执行记录不存在".to_string())
    }
}

/// 获取子任务执行详情（UI展示用，不需要鉴权）
#[tauri::command]
pub async fn get_sub_task_execution_detail_for_ui(
    app_handle: tauri::AppHandle,
    execution_id: i64,
) -> Result<Option<SubTaskExecution>, String> {
    let db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let exec_repo = db.execution_repo().map_err(|e| e.to_string())?;

    // 直接获取执行详情，不进行鉴权检查（用于UI展示）
    let execution = exec_repo.read(execution_id).map_err(|e| e.to_string())?;
    Ok(execution)
}

/// 取消子任务执行（UI专用，不需要鉴权）
#[tauri::command]
pub async fn cancel_sub_task_execution_for_ui(
    app_handle: tauri::AppHandle,
    execution_id: i64,
) -> Result<(), String> {
    let db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let exec_repo = db.execution_repo().map_err(|e| e.to_string())?;

    if let Some(execution) = exec_repo.read(execution_id).map_err(|e| e.to_string())? {
        // 只有 pending 或 running 状态的任务可以取消
        if execution.status != "pending" && execution.status != "running" {
            return Err(format!("任务状态为 '{}' 时无法取消", execution.status));
        }

        // 更新状态为 cancelled
        exec_repo
            .update_status(execution_id, "cancelled", None)
            .map_err(|e| e.to_string())?;

        // 发送状态更新事件
        if let Ok(Some(updated_execution)) = exec_repo.read(execution_id) {
            emit_sub_task_status_update(&app_handle, &updated_execution).await;
        }

        Ok(())
    } else {
        Err("任务执行记录不存在".to_string())
    }
}