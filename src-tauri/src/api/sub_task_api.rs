use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::Emitter;

use crate::{
    api::{
        ai::{
            config::{get_network_proxy_from_config, get_request_timeout_from_config},
            conversation::build_chat_messages,
        },
        assistant_api::get_assistant,
        genai_client::create_client_with_config,
    },
    db::{
        conversation_db::{ConversationDatabase, Repository as ConversationRepository},
        llm_db::LLMDatabase,
        sub_task_db::{
            SubTaskDatabase, SubTaskDefinition, SubTaskExecution, SubTaskExecutionSummary,
        },
    },
    mcp::{
        detection::detect_and_process_mcp_calls_for_subtask,
        mcp_db::MCPToolCall,
        prompt::{collect_mcp_info_for_assistant, format_mcp_prompt_with_filters},
    },
    FeatureConfigState,
};
use genai::chat::{ChatOptions, ChatRequest};
use tauri::State;

// MCP 循环选项
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct McpLoopOptions {
    // 允许哪些 server 的 name
    pub enabled_servers: Vec<String>,

    // 针对特定 server 限制工具（key=serverName, value=允许的 tool 名称列表）
    pub enabled_tools: Option<HashMap<String, Vec<String>>>,

    // 最大循环轮数（模型 ↔ 工具往返），默认 3
    pub max_loops: Option<u32>,

    // 单个工具执行超时（毫秒），默认 60000 ms
    pub tool_timeout_ms: Option<u32>,

    // mcp提示词的注入位置，默认 append
    pub mcp_prompt_injection_mode: Option<String>, // 'append' | 'prepend'

    // 遇到工具执行错误是否继续后续工具 默认false
    pub continue_on_tool_error: Option<bool>,

    // 如果循环达到 maxLoops 仍有调用请求，是否强制终止 默认true
    pub hard_stop_on_max_loops: Option<bool>,

    // 启用调试日志（供外层 UI 展示），默认false
    pub debug: Option<bool>,
}

// MCP 循环结果
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct McpLoopResult {
    // 注：最终留给上层业务消费的文本（可作为 parseSearchResults 的输入等）
    pub final_text: String,

    // 最后一轮模型的原始文本（含工具标签，便于调试或二次解析）
    pub raw_model_output: String,

    // 所有解析出的调用（按时间序）
    pub calls: Vec<MCPToolCall>,

    // 实际循环了多少轮
    pub loops: u32,

    // 是否因为达到 maxLoops 中止
    pub reached_max_loops: bool,

    // 中止/失败原因（如 'abort_by_interceptor' | 'hard_stop' | 'no_calls' 等）
    pub abort_reason: Option<String>,

    // 指标统计
    pub metrics: McpLoopMetrics,

    // 额外调试信息
    pub debug_log: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct McpLoopMetrics {
    pub total_calls: u32,
    pub success_calls: u32,
    pub failed_calls: u32,
    pub total_exec_time_ms: u64,
    pub average_exec_time_ms: u64,
}

// 扩展子任务运行结果，包含 MCP 执行信息
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubTaskRunWithMcpResult {
    pub success: bool,
    pub content: Option<String>,
    pub error: Option<String>,
    pub execution_id: i64,
    pub mcp_result: Option<McpLoopResult>,
}

// 事件定义
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct SubTaskExecutionParams {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<i32>,
    pub custom_model_id: Option<i64>,
}

// 创建子任务请求
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
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
            let server = mcp_db
                .get_mcp_server(source_id)
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
async fn emit_sub_task_status_update(app_handle: &tauri::AppHandle, execution: &SubTaskExecution) {
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

    let _ =
        app_handle.emit(&format!("sub_task_update_{}", execution.parent_conversation_id), event);
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

    // 检查 code 是否已存在
    if db.find_definition_by_code(&code).map_err(|e| e.to_string())?.is_some() {
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

    let created = db.create_sub_task_definition(&definition).map_err(|e| e.to_string())?;
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

    let definitions = db
        .list_definitions_by_source(plugin_source.as_deref(), source_id, is_enabled)
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

    if let Some(definition) = db.find_definition_by_code(&code).map_err(|e| e.to_string())? {
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

    // 获取现有定义并检查权限
    if let Some(mut definition) = db.read_sub_task_definition(id).map_err(|e| e.to_string())? {
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

        db.update_sub_task_definition(&definition).map_err(|e| e.to_string())?;
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

    // 获取现有定义并检查权限
    if let Some(definition) = db.read_sub_task_definition(id).map_err(|e| e.to_string())? {
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

        db.delete_sub_task_definition_row(id).map_err(|e| e.to_string())?;
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

    let task_definition = sub_task_db
        .find_definition_by_code(&request.task_code)
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

    if conv_repo.read(request.parent_conversation_id).map_err(|e| e.to_string())?.is_none() {
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

    let created_execution =
        sub_task_db.create_sub_task_execution(&execution).map_err(|e| e.to_string())?;
    let execution_id = created_execution.id;

    // 异步执行任务
    let app_handle_clone = app_handle.clone();
    let task_def_clone = task_definition.clone();
    let task_prompt_clone = request.task_prompt.clone();
    let _ai_params_clone = request.ai_params.clone();

    tokio::spawn(async move {
        // 更新状态为 running
        let sub_task_db = SubTaskDatabase::new(&app_handle_clone).unwrap();
        let started_time = Utc::now();

        let _ = sub_task_db.update_execution_status(execution_id, "running", Some(started_time));

        // 发送状态更新事件
        let mut updated_execution =
            sub_task_db.read_sub_task_execution(execution_id).unwrap().unwrap();
        updated_execution.status = "running".to_string();
        updated_execution.started_time = Some(started_time);
        emit_sub_task_status_update(&app_handle_clone, &updated_execution).await;

        // 简化执行任务：暂时返回固定结果
        let result: Result<(String, Option<(i32, i32, i32)>), String> = Ok((
            format!("执行任务 '{}' 完成，输入: {}", task_def_clone.name, task_prompt_clone),
            Some((100, 50, 50)),
        ));

        // 更新执行结果
        let finished_time = Utc::now();
        match result {
            Ok((content, token_stats)) => {
                let _ = sub_task_db.update_execution_result(
                    execution_id,
                    "success",
                    Some(&content),
                    None,
                    token_stats,
                    Some(finished_time),
                );
            }
            Err(error) => {
                let _ = sub_task_db.update_execution_result(
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
        if let Ok(Some(final_execution)) = sub_task_db.read_sub_task_execution(execution_id) {
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
    status: Option<String>, // 过滤条件
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<Vec<SubTaskExecutionSummary>, String> {
    let db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);

    let executions = db
        .list_executions_by_conversation(
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

    if let Some(execution) = db.read_sub_task_execution(execution_id).map_err(|e| e.to_string())? {
        // 获取任务定义进行鉴权检查
        if let Some(definition) =
            db.read_sub_task_definition(execution.task_definition_id).map_err(|e| e.to_string())?
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

    if let Some(execution) = db.read_sub_task_execution(execution_id).map_err(|e| e.to_string())? {
        // 获取任务定义进行鉴权检查
        if let Some(definition) =
            db.read_sub_task_definition(execution.task_definition_id).map_err(|e| e.to_string())?
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
            db.update_execution_status(execution_id, "cancelled", None)
                .map_err(|e| e.to_string())?;

            // 发送状态更新事件
            if let Ok(Some(updated_execution)) = db.read_sub_task_execution(execution_id) {
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

    // 直接获取执行详情，不进行鉴权检查（用于UI展示）
    let execution = db.read_sub_task_execution(execution_id).map_err(|e| e.to_string())?;
    Ok(execution)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubTaskRunResult {
    pub success: bool,
    pub content: Option<String>,
    pub error: Option<String>,
    pub execution_id: i64,
}

#[tauri::command]
pub async fn run_sub_task_sync(
    app_handle: tauri::AppHandle,
    feature_config_state: State<'_, FeatureConfigState>,
    code: String,
    task_prompt: String,
    conversation_id: i64,
    assistant_id: i64,
) -> Result<SubTaskRunResult, String> {
    // 获取任务定义
    let sub_task_db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    let task_definition = sub_task_db
        .find_definition_by_code(&code)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Sub task '{}' not found", code))?;

    // 检查任务是否启用
    if !task_definition.is_enabled {
        return Err("Sub task is disabled".to_string());
    }

    // 验证父对话是否存在
    let conv_db = ConversationDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let conv_repo = conv_db.conversation_repo().map_err(|e| e.to_string())?;

    if conv_repo.read(conversation_id).map_err(|e| e.to_string())?.is_none() {
        return Err("Conversation not found".to_string());
    }

    // 获取助手配置
    let assistant_detail = get_assistant(app_handle.clone(), assistant_id)
        .map_err(|e| format!("Failed to get assistant: {}", e))?;

    // 获取特征配置
    let config_feature_map = feature_config_state.config_feature_map.lock().await;
    let config_map = config_feature_map.clone();
    drop(config_feature_map);

    // 创建执行记录
    let execution = SubTaskExecution {
        id: 0,
        task_definition_id: task_definition.id,
        task_code: code.clone(),
        task_name: task_definition.name.clone(),
        task_prompt: task_prompt.clone(),
        parent_conversation_id: conversation_id,
        parent_message_id: None, // Run from plugin context
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

    let created_execution =
        sub_task_db.create_sub_task_execution(&execution).map_err(|e| e.to_string())?;
    let execution_id = created_execution.id;

    // 同步执行任务
    let started_time = Utc::now();
    let _ = sub_task_db.update_execution_status(execution_id, "running", Some(started_time));

    // 发送状态更新事件
    let mut updated_execution =
        sub_task_db.read_sub_task_execution(execution_id).map_err(|e| e.to_string())?.unwrap();
    updated_execution.status = "running".to_string();
    updated_execution.started_time = Some(started_time);
    emit_sub_task_status_update(&app_handle, &updated_execution).await;

    // 实际执行AI任务
    let result: Result<(String, Option<(i32, i32, i32)>), String> = {
        // 获取LLM数据库连接获取模型配置
        let llm_db = LLMDatabase::new(&app_handle).map_err(|e| e.to_string())?;

        // 获取助手的模型信息
        let model_info = if assistant_detail.model.is_empty() {
            return Err("Assistant has no model configured".to_string());
        } else {
            &assistant_detail.model[0]
        };

        // 按提供商ID + 模型代码定位模型（避免将 provider_id 误当作 llm_model.id）
        let llm_model = llm_db
            .get_llm_model_detail(&model_info.provider_id, &model_info.model_code)
            .map_err(|e| {
                format!(
                    "Failed to get LLM model (provider_id={}, code={}): {}",
                    model_info.provider_id, model_info.model_code, e
                )
            })?;

        let model_name = if !model_info.model_code.is_empty() {
            &model_info.model_code
        } else {
            return Err("Model code is empty".to_string());
        };

        // 获取提供商配置
        let provider_configs = llm_db
            .get_llm_provider_config(model_info.provider_id)
            .map_err(|e| format!("Failed to get provider config: {}", e))?;

        // 构建配置
        let network_proxy = get_network_proxy_from_config(&config_map);
        let request_timeout = get_request_timeout_from_config(&config_map);
        let proxy_enabled = network_proxy.is_some(); // 简化proxy启用检查

        // 创建AI客户端
        let client = create_client_with_config(
            &provider_configs,
            model_name,
            &llm_model.provider.api_type,
            network_proxy.as_deref(),
            proxy_enabled,
            Some(request_timeout), // 包装为Option
        )
        .map_err(|e| format!("Failed to create AI client: {}", e))?;

        // 构建消息
        let init_messages = vec![
            ("system".to_string(), task_definition.system_prompt.clone(), vec![]),
            ("user".to_string(), task_prompt.clone(), vec![]),
        ];

        let chat_messages = build_chat_messages(&init_messages);
        let chat_request = ChatRequest::new(chat_messages);

        // 构建聊天选项
        let mut chat_options = ChatOptions::default();

        // 应用助手的模型配置
        for config in &assistant_detail.model_configs {
            match config.name.as_str() {
                "max_tokens" => {
                    if let Some(value) = &config.value {
                        if let Ok(max_tokens) = value.parse::<u32>() {
                            chat_options = chat_options.with_max_tokens(max_tokens);
                        }
                    }
                }
                "temperature" => {
                    if let Some(value) = &config.value {
                        if let Ok(temperature) = value.parse::<f64>() {
                            chat_options = chat_options.with_temperature(temperature);
                        }
                    }
                }
                "top_p" => {
                    if let Some(value) = &config.value {
                        if let Ok(top_p) = value.parse::<f64>() {
                            chat_options = chat_options.with_top_p(top_p);
                        }
                    }
                }
                _ => {}
            }
        }

        // 执行AI调用
        match client.exec_chat(model_name, chat_request, Some(&chat_options)).await {
            Ok(response) => {
                // 提取响应内容
                let content = if response.content.is_empty() {
                    String::new()
                } else {
                    response
                        .content
                        .into_iter()
                        .map(|c| match c {
                            genai::chat::MessageContent::Text(text) => text,
                            _ => String::new(),
                        })
                        .collect::<Vec<_>>()
                        .join("")
                };

                let token_usage = response.usage;

                // 提取token统计
                let token_stats = {
                    let total = token_usage.total_tokens.unwrap_or(0) as i32;
                    let input = token_usage.prompt_tokens.unwrap_or(0) as i32;
                    let output = token_usage.completion_tokens.unwrap_or(0) as i32;
                    Some((total, input, output))
                };

                Ok((content, token_stats))
            }
            Err(e) => Err(format!("AI execution failed: {}", e)),
        }
    };

    // 更新执行结果
    let finished_time = Utc::now();
    let sub_task_result = match result {
        Ok((content, token_stats)) => {
            let _ = sub_task_db.update_execution_result(
                execution_id,
                "success",
                Some(&content),
                None,
                token_stats,
                Some(finished_time),
            );
            SubTaskRunResult { success: true, content: Some(content), error: None, execution_id }
        }
        Err(error) => {
            let _ = sub_task_db.update_execution_result(
                execution_id,
                "failed",
                None,
                Some(&error),
                None,
                Some(finished_time),
            );
            SubTaskRunResult { success: false, content: None, error: Some(error), execution_id }
        }
    };

    // 发送完成事件
    if let Ok(Some(final_execution)) = sub_task_db.read_sub_task_execution(execution_id) {
        emit_sub_task_status_update(&app_handle, &final_execution).await;
    }

    Ok(sub_task_result)
}

#[tauri::command]
pub async fn sub_task_regist(
    app_handle: tauri::AppHandle,
    code: String,
    name: String,
    description: String,
    system_prompt: String,
    plugin_source: String,
    source_id: i64,
) -> Result<i64, String> {
    let db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    let definition = SubTaskDefinition {
        id: 0, // Will be set by upsert_definition
        name,
        code,
        description,
        system_prompt,
        plugin_source,
        source_id,
        is_enabled: true, // Default enabled
        created_time: Utc::now(),
        updated_time: Utc::now(),
    };

    let result = db.upsert_sub_task_definition(&definition).map_err(|e| e.to_string())?;
    Ok(result.id)
}

/// 取消子任务执行（UI专用，不需要鉴权）
#[tauri::command]
pub async fn cancel_sub_task_execution_for_ui(
    app_handle: tauri::AppHandle,
    execution_id: i64,
) -> Result<(), String> {
    let db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    if let Some(execution) = db.read_sub_task_execution(execution_id).map_err(|e| e.to_string())? {
        // 只有 pending 或 running 状态的任务可以取消
        if execution.status != "pending" && execution.status != "running" {
            return Err(format!("任务状态为 '{}' 时无法取消", execution.status));
        }

        // 更新状态为 cancelled
        db.update_execution_status(execution_id, "cancelled", None).map_err(|e| e.to_string())?;

        // 发送状态更新事件
        if let Ok(Some(updated_execution)) = db.read_sub_task_execution(execution_id) {
            emit_sub_task_status_update(&app_handle, &updated_execution).await;
        }

        Ok(())
    } else {
        Err("任务执行记录不存在".to_string())
    }
}

/// 核心 MCP 循环执行引擎
async fn execute_mcp_loop(
    app_handle: &tauri::AppHandle,
    subtask_id: i64,
    conversation_id: i64,
    assistant_id: i64,
    system_prompt: &str,
    user_prompt: &str,
    options: &McpLoopOptions,
    config_map: &HashMap<String, HashMap<String, crate::db::system_db::FeatureConfig>>,
) -> Result<McpLoopResult, String> {
    let max_loops = options.max_loops.unwrap_or(3);
    let debug = options.debug.unwrap_or(false);
    let injection_mode = options.mcp_prompt_injection_mode.as_deref().unwrap_or("append");

    let mut debug_log = if debug { Some(Vec::new()) } else { None };
    let mut all_calls = Vec::new();

    // Collect MCP info for prompt injection
    let mcp_info = if injection_mode != "none" {
        Some(
            collect_mcp_info_for_assistant(
                app_handle,
                assistant_id,
                None,
                Some(&options.enabled_servers),
            )
            .await
            .map_err(|e| format!("Failed to collect MCP info: {}", e))?,
        )
    } else {
        None
    };

    // Build initial messages with MCP prompt injection
    let mut current_messages = vec![];

    match injection_mode {
        "prepend" => {
            // Add system prompt first
            current_messages.push(("system".to_string(), system_prompt.to_string(), vec![]));

            // Inject MCP prompt after system prompt if MCP info is available
            if let Some(ref mcp_info) = mcp_info {
                let mcp_prompt = format_mcp_prompt_with_filters(
                    "".to_string(),
                    mcp_info,
                    Some(&options.enabled_servers),
                    options.enabled_tools.as_ref(),
                )
                .await;
                current_messages.push(("system".to_string(), mcp_prompt, vec![]));
            }

            // Add user prompt
            current_messages.push(("user".to_string(), user_prompt.to_string(), vec![]));
        }
        "append" => {
            // Build enhanced system prompt by appending MCP prompt
            let enhanced_system_prompt = if let Some(ref mcp_info) = mcp_info {
                format_mcp_prompt_with_filters(
                    system_prompt.to_string(),
                    mcp_info,
                    Some(&options.enabled_servers),
                    options.enabled_tools.as_ref(),
                )
                .await
            } else {
                system_prompt.to_string()
            };

            current_messages.push(("system".to_string(), enhanced_system_prompt, vec![]));
            current_messages.push(("user".to_string(), user_prompt.to_string(), vec![]));
        }
        _ => {
            // Default behavior - no MCP prompt injection
            current_messages.push(("system".to_string(), system_prompt.to_string(), vec![]));
            current_messages.push(("user".to_string(), user_prompt.to_string(), vec![]));
        }
    }

    let mut loops_count = 0u32;
    let mut final_text = String::new();
    let mut raw_model_output = String::new();
    let loop_start_time = std::time::Instant::now();

    if let Some(ref mut log) = debug_log {
        log.push(format!("开始 MCP 循环执行，最大循环数: {}", max_loops));
        log.push(format!("MCP 提示词注入模式: {}", injection_mode));
        if let Some(ref mcp_info) = mcp_info {
            log.push(format!("可用 MCP 服务器数量: {}", mcp_info.enabled_servers.len()));
            log.push(format!("限制的服务器: {:?}", options.enabled_servers));
            if let Some(ref enabled_tools) = options.enabled_tools {
                log.push(format!("限制的工具: {:?}", enabled_tools));
            }
        }
    }

    // 获取助手配置
    let assistant_detail = get_assistant(app_handle.clone(), assistant_id)
        .map_err(|e| format!("Failed to get assistant: {}", e))?;

    // 获取模型信息
    let model_info = if assistant_detail.model.is_empty() {
        return Err("Assistant has no model configured".to_string());
    } else {
        &assistant_detail.model[0]
    };

    // 获取 LLM 数据库连接
    let llm_db = LLMDatabase::new(app_handle).map_err(|e| e.to_string())?;
    let llm_model = llm_db
        .get_llm_model_detail(&model_info.provider_id, &model_info.model_code)
        .map_err(|e| {
            format!(
                "Failed to get LLM model (provider_id={}, code={}): {}",
                model_info.provider_id, model_info.model_code, e
            )
        })?;

    let model_name = &model_info.model_code;
    let provider_configs = llm_db
        .get_llm_provider_config(model_info.provider_id)
        .map_err(|e| format!("Failed to get provider config: {}", e))?;

    // 构建客户端配置
    let network_proxy = get_network_proxy_from_config(config_map);
    let request_timeout = get_request_timeout_from_config(config_map);
    let proxy_enabled = network_proxy.is_some();

    let client = create_client_with_config(
        &provider_configs,
        model_name,
        &llm_model.provider.api_type,
        network_proxy.as_deref(),
        proxy_enabled,
        Some(request_timeout),
    )
    .map_err(|e| format!("Failed to create AI client: {}", e))?;

    // 构建聊天选项
    let mut chat_options = ChatOptions::default();
    for config in &assistant_detail.model_configs {
        match config.name.as_str() {
            "max_tokens" => {
                if let Some(value) = &config.value {
                    if let Ok(max_tokens) = value.parse::<u32>() {
                        chat_options = chat_options.with_max_tokens(max_tokens);
                    }
                }
            }
            "temperature" => {
                if let Some(value) = &config.value {
                    if let Ok(temperature) = value.parse::<f64>() {
                        chat_options = chat_options.with_temperature(temperature);
                    }
                }
            }
            "top_p" => {
                if let Some(value) = &config.value {
                    if let Ok(top_p) = value.parse::<f64>() {
                        chat_options = chat_options.with_top_p(top_p);
                    }
                }
            }
            _ => {}
        }
    }

    // MCP 工具循环
    loop {
        if loops_count >= max_loops {
            if let Some(ref mut log) = debug_log {
                log.push(format!("达到最大循环数: {}", max_loops));
            }
            break;
        }

        loops_count += 1;

        if let Some(ref mut log) = debug_log {
            log.push(format!("开始第 {} 轮循环", loops_count));
        }

        // 执行 AI 调用
        let chat_messages = build_chat_messages(&current_messages);
        let chat_request = ChatRequest::new(chat_messages);

        let ai_response =
            match client.exec_chat(model_name, chat_request, Some(&chat_options)).await {
                Ok(response) => {
                    let content = if response.content.is_empty() {
                        String::new()
                    } else {
                        response
                            .content
                            .into_iter()
                            .map(|c| match c {
                                genai::chat::MessageContent::Text(text) => text,
                                _ => String::new(),
                            })
                            .collect::<Vec<_>>()
                            .join("")
                    };
                    content
                }
                Err(e) => {
                    return Err(format!("AI execution failed: {}", e));
                }
            };

        raw_model_output = ai_response.clone();
        final_text = ai_response.clone();

        if let Some(ref mut log) = debug_log {
            log.push(format!("AI 响应: {}", ai_response));
        }

        // 检测并执行 MCP 工具调用（最大化复用现有逻辑）
        let executed_calls = detect_and_process_mcp_calls_for_subtask(
            app_handle,
            conversation_id,
            subtask_id,
            &ai_response,
            &options.enabled_servers,
            &options.enabled_tools,
        )
        .await
        .map_err(|e| format!("MCP call detection failed: {}", e))?;

        if executed_calls.is_empty() {
            if let Some(ref mut log) = debug_log {
                log.push("没有检测到 MCP 工具调用，结束循环".to_string());
            }
            break;
        }

        if let Some(ref mut log) = debug_log {
            log.push(format!("检测并执行了 {} 个 MCP 工具调用", executed_calls.len()));
        }

        // 将执行的调用添加到记录中
        all_calls.extend(executed_calls.clone());

        // 构建工具结果文本
        let mut tool_results = Vec::new();
        for call in executed_calls {
            if let Some(ref mut log) = debug_log {
                log.push(format!(
                    "工具调用: {} / {} - 状态: {}",
                    call.server_name, call.tool_name, call.status
                ));
            }

            let result_text = if call.status == "success" {
                format!(
                    "Tool: {}\nServer: {}\nParameters: {}\nResult:\n{}",
                    call.tool_name,
                    call.server_name,
                    call.parameters,
                    call.result.as_deref().unwrap_or("No result")
                )
            } else {
                let error_msg = call.error.as_deref().unwrap_or("Unknown error");

                if !options.continue_on_tool_error.unwrap_or(false) {
                    return Err(format!("Tool execution failed: {}", error_msg));
                }

                format!(
                    "Tool: {}\nServer: {}\nParameters: {}\nError: {}",
                    call.tool_name, call.server_name, call.parameters, error_msg
                )
            };

            tool_results.push(result_text);
        }

        // 将工具结果添加到对话历史
        if !tool_results.is_empty() {
            current_messages.push((
                "user".to_string(),
                format!("Tool execution results:\n\n{}", tool_results.join("\n\n")),
                vec![],
            ));
        }
    }

    let total_time = loop_start_time.elapsed().as_millis() as u64;
    let success_calls = all_calls.iter().filter(|c| c.status == "success").count() as u32;
    let failed_calls = all_calls.iter().filter(|c| c.status == "failed").count() as u32;

    let metrics = McpLoopMetrics {
        total_calls: all_calls.len() as u32,
        success_calls,
        failed_calls,
        total_exec_time_ms: total_time,
        average_exec_time_ms: if all_calls.is_empty() {
            0
        } else {
            total_time / all_calls.len() as u64
        },
    };

    println!("[SubTask] MCP Logs: {:?}", debug_log);

    Ok(McpLoopResult {
        final_text,
        raw_model_output,
        calls: all_calls,
        loops: loops_count,
        reached_max_loops: loops_count >= max_loops,
        abort_reason: None,
        metrics,
        debug_log,
    })
}

#[tauri::command]
pub async fn run_sub_task_with_mcp_loop(
    app_handle: tauri::AppHandle,
    feature_config_state: State<'_, FeatureConfigState>,
    code: String,
    task_prompt: String,
    conversation_id: i64,
    assistant_id: i64,
    options: McpLoopOptions,
) -> Result<SubTaskRunWithMcpResult, String> {
    // 获取任务定义
    let sub_task_db = SubTaskDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    let task_definition = sub_task_db
        .find_definition_by_code(&code)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Sub task '{}' not found", code))?;

    // 检查任务是否启用
    if !task_definition.is_enabled {
        return Err("Sub task is disabled".to_string());
    }

    // 验证父对话是否存在
    let conv_db = ConversationDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let conv_repo = conv_db.conversation_repo().map_err(|e| e.to_string())?;

    if conv_repo.read(conversation_id).map_err(|e| e.to_string())?.is_none() {
        return Err("Conversation not found".to_string());
    }

    // 获取特征配置
    let config_feature_map = feature_config_state.config_feature_map.lock().await;
    let config_map = config_feature_map.clone();
    drop(config_feature_map);

    // 创建执行记录
    let execution = SubTaskExecution {
        id: 0,
        task_definition_id: task_definition.id,
        task_code: code.clone(),
        task_name: task_definition.name.clone(),
        task_prompt: task_prompt.clone(),
        parent_conversation_id: conversation_id,
        parent_message_id: None, // MCP loop context
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

    let created_execution =
        sub_task_db.create_sub_task_execution(&execution).map_err(|e| e.to_string())?;
    let execution_id = created_execution.id;

    // 执行 MCP 循环
    let started_time = Utc::now();
    let _ = sub_task_db.update_execution_status(execution_id, "running", Some(started_time));

    // 发送状态更新事件
    let mut updated_execution =
        sub_task_db.read_sub_task_execution(execution_id).map_err(|e| e.to_string())?.unwrap();
    updated_execution.status = "running".to_string();
    updated_execution.started_time = Some(started_time);
    emit_sub_task_status_update(&app_handle, &updated_execution).await;

    let mcp_result = execute_mcp_loop(
        &app_handle,
        execution_id,
        conversation_id,
        assistant_id,
        &task_definition.system_prompt,
        &task_prompt,
        &options,
        &config_map,
    )
    .await;

    let finished_time = Utc::now();
    let sub_task_result = match mcp_result {
        Ok(mcp_loop_result) => {
            let _ = sub_task_db.update_execution_result(
                execution_id,
                "success",
                Some(&mcp_loop_result.final_text),
                None,
                Some((0, 0, 0)), // Token stats not tracked for MCP loops yet
                Some(finished_time),
            );

            SubTaskRunWithMcpResult {
                success: true,
                content: Some(mcp_loop_result.final_text.clone()),
                error: None,
                execution_id,
                mcp_result: Some(mcp_loop_result),
            }
        }
        Err(error) => {
            let _ = sub_task_db.update_execution_result(
                execution_id,
                "failed",
                None,
                Some(&error),
                None,
                Some(finished_time),
            );

            SubTaskRunWithMcpResult {
                success: false,
                content: None,
                error: Some(error),
                execution_id,
                mcp_result: None,
            }
        }
    };

    // 发送完成事件
    if let Ok(Some(final_execution)) = sub_task_db.read_sub_task_execution(execution_id) {
        emit_sub_task_status_update(&app_handle, &final_execution).await;
    }

    Ok(sub_task_result)
}
