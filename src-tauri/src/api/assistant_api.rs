use crate::{
    db::{
        assistant_db::{
            Assistant, AssistantDatabase, AssistantMCPConfig, AssistantMCPToolConfig,
            AssistantModel, AssistantModelConfig, AssistantPrompt, AssistantPromptParam,
        },
        conversation_db::ConversationDatabase,
    },
    NameCacheState,
};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct AssistantDetail {
    pub assistant: Assistant,
    pub prompts: Vec<AssistantPrompt>,
    pub model: Vec<AssistantModel>,
    pub model_configs: Vec<AssistantModelConfig>,
    pub prompt_params: Vec<AssistantPromptParam>,
    pub mcp_configs: Vec<AssistantMCPConfig>,
    pub mcp_tool_configs: Vec<AssistantMCPToolConfig>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct MCPServerInfo {
    pub id: i64,
    pub name: String,
    pub is_enabled: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct MCPToolInfo {
    pub id: i64,
    pub name: String,
    pub is_enabled: bool,
    pub is_auto_run: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct MCPServerWithTools {
    pub id: i64,
    pub name: String,
    pub is_enabled: bool,
    pub tools: Vec<MCPToolInfo>,
}

#[tauri::command]
pub fn get_assistants(app_handle: tauri::AppHandle) -> Result<Vec<Assistant>, String> {
    let assistant_db = AssistantDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    assistant_db
        .get_assistants()
        .map(|assistants| assistants.into())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_assistant(
    app_handle: tauri::AppHandle,
    assistant_id: i64,
) -> Result<AssistantDetail, String> {
    let assistant_db = AssistantDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    // 获取 Assistant 基本信息
    let assistant = assistant_db
        .get_assistant(assistant_id)
        .map_err(|e| e.to_string())?;
    println!(
        "Assistant name: {:?} (id: {})",
        assistant.name, assistant.id
    );

    // 获取相关的 prompt
    let prompts = assistant_db
        .get_assistant_prompt(assistant_id)
        .map_err(|e| e.to_string())?;
    println!("prompts: {:?}", prompts);

    // 获取相关的 model
    let model = assistant_db
        .get_assistant_model(assistant_id)
        .map_err(|e| e.to_string())?;
    println!("model: {:?}", model);

    // 获取相关的 model_config
    let model_configs = assistant_db
        .get_assistant_model_configs(assistant_id)
        .map_err(|e| e.to_string())?;
    println!("model_configs: {:?}", model_configs);

    // 获取相关的 prompt_params
    let prompt_params = assistant_db
        .get_assistant_prompt_params(assistant_id)
        .map_err(|e| e.to_string())?;
    println!("prompt_params: {:?}", prompt_params);

    // 获取相关的 MCP 配置
    let mcp_configs = assistant_db
        .get_assistant_mcp_configs(assistant_id)
        .map_err(|e| e.to_string())?;
    println!("mcp_configs: {:?}", mcp_configs);

    // 获取相关的 MCP 工具配置
    let mcp_tool_configs = assistant_db
        .get_assistant_mcp_tool_configs(assistant_id)
        .map_err(|e| e.to_string())?;
    println!("mcp_tool_configs: {:?}", mcp_tool_configs);

    // 构建 AssistantDetail 对象
    let assistant_detail = AssistantDetail {
        assistant,
        prompts,
        model,
        model_configs,
        prompt_params,
        mcp_configs,
        mcp_tool_configs,
    };

    Ok(assistant_detail)
}

#[tauri::command]
pub async fn save_assistant(
    app_handle: tauri::AppHandle,
    name_cache_state: tauri::State<'_, NameCacheState>,
    assistant_detail: AssistantDetail,
) -> Result<(), String> {
    let assistant_db = AssistantDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    println!(
        "save_assistant assistant_detail: {:?}",
        assistant_detail.clone()
    );

    // Save or update the Assistant
    if assistant_detail.assistant.id == 0 {
        assistant_db
            .add_assistant(
                &assistant_detail.assistant.name,
                assistant_detail
                    .assistant
                    .description
                    .as_deref()
                    .unwrap_or(""),
                assistant_detail.assistant.assistant_type,
                true,
            )
            .map_err(|e| e.to_string())?;
    } else {
        assistant_db
            .update_assistant(
                assistant_detail.assistant.id,
                &assistant_detail.assistant.name,
                assistant_detail
                    .assistant
                    .description
                    .as_deref()
                    .unwrap_or(""),
            )
            .map_err(|e| e.to_string())?;
    }

    // Update the name_cache_state
    let mut model_names = name_cache_state.assistant_names.lock().await;
    model_names.insert(
        assistant_detail.assistant.id,
        assistant_detail.assistant.name,
    );

    // Save or update the AssistantPrompts
    for prompt in assistant_detail.prompts {
        if prompt.id == 0 {
            assistant_db
                .add_assistant_prompt(prompt.assistant_id, &prompt.prompt)
                .map_err(|e| e.to_string())?;
        } else {
            assistant_db
                .update_assistant_prompt(prompt.id, &prompt.prompt)
                .map_err(|e| e.to_string())?;
        }
    }

    // Save or update the AssistantModels
    for mut model in assistant_detail.model {
        if model.id == 0 {
            let result_id = assistant_db
                .add_assistant_model(
                    model.assistant_id,
                    model.provider_id,
                    &model.model_code,
                    &model.alias,
                )
                .map_err(|e| e.to_string())?;
            model.id = result_id;
        } else {
            assistant_db
                .update_assistant_model(
                    model.id,
                    model.provider_id,
                    &model.model_code,
                    &model.alias,
                )
                .map_err(|e| e.to_string())?;
        }
    }

    // Save or update the AssistantModelConfigs
    for mut config in assistant_detail.model_configs {
        if config.id == 0 {
            let result_id = assistant_db
                .add_assistant_model_config(
                    config.assistant_id,
                    config.id,
                    &config.name,
                    config.value.as_deref().unwrap_or(""),
                    &config.value_type,
                )
                .map_err(|e| e.to_string())?;
            config.id = result_id;
        } else {
            assistant_db
                .update_assistant_model_config(
                    config.id,
                    &config.name,
                    config.value.as_deref().unwrap_or(""),
                )
                .map_err(|e| e.to_string())?;
        }
    }

    // Save or update the AssistantPromptParams
    for param in assistant_detail.prompt_params {
        if param.id == 0 {
            assistant_db
                .add_assistant_prompt_param(
                    param.assistant_id,
                    param.assistant_prompt_id,
                    &param.param_name,
                    param.param_type.as_deref().unwrap_or(""),
                    param.param_value.as_deref().unwrap_or(""),
                )
                .map_err(|e| e.to_string())?;
        } else {
            assistant_db
                .update_assistant_prompt_param(
                    param.id,
                    &param.param_name,
                    param.param_type.as_deref().unwrap_or(""),
                    param.param_value.as_deref().unwrap_or(""),
                )
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[tauri::command]
pub fn add_assistant(
    app_handle: tauri::AppHandle,
    name: String,
    description: String,
    assistant_type: i64,
) -> Result<AssistantDetail, String> {
    println!("start add assistant");
    let assistant_db = AssistantDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    // Add a default assistant
    let assistant_id = assistant_db
        .add_assistant(&name, &description, Some(assistant_type), false)
        .map_err(|e| e.to_string())?;

    // Get the newly added assistant
    let assistant = assistant_db
        .get_assistant(assistant_id)
        .map_err(|e| e.to_string())?;
    println!("assistant: {:?}", assistant);

    let default_prompt = "You are a helpful assistant.";
    let prompt_id = assistant_db
        .add_assistant_prompt(assistant_id, default_prompt)
        .map_err(|e| e.to_string())?;
    let prompts = vec![AssistantPrompt {
        id: prompt_id,
        assistant_id: assistant_id,
        prompt: default_prompt.to_string(),
        created_time: Option::None,
    }];

    let model_id = assistant_db
        .add_assistant_model(assistant_id, 0, "", "")
        .map_err(|e| e.to_string())?;
    println!("model_id: {:?}", model_id);

    // Add default model configs
    let default_model_configs = vec![
        AssistantModelConfig {
            id: 0,
            assistant_id,
            assistant_model_id: model_id, // Assuming 0 is a default model ID
            name: "max_tokens".to_string(),
            value: Some("2000".to_string()),
            value_type: "number".to_string(),
        },
        AssistantModelConfig {
            id: 0,
            assistant_id,
            assistant_model_id: model_id, // Assuming 0 is a default model ID
            name: "temperature".to_string(),
            value: Some("0.7".to_string()),
            value_type: "float".to_string(),
        },
        AssistantModelConfig {
            id: 0,
            assistant_id,
            assistant_model_id: model_id, // Assuming 0 is a default model ID
            name: "top_p".to_string(),
            value: Some("1.0".to_string()),
            value_type: "float".to_string(),
        },
        AssistantModelConfig {
            id: 0,
            assistant_id,
            assistant_model_id: model_id, // Assuming 0 is a default model ID
            name: "stream".to_string(),
            value: Some("true".to_string()),
            value_type: "boolean".to_string(),
        },
    ];
    let mut model_configs = Vec::new();
    for config in default_model_configs {
        let config_id = assistant_db
            .add_assistant_model_config(
                config.assistant_id,
                config.assistant_model_id,
                &config.name,
                config.value.as_deref().unwrap_or(""),
                &config.value_type,
            )
            .map_err(|e| e.to_string())?;
        model_configs.push(AssistantModelConfig {
            id: config_id,
            assistant_id: config.assistant_id,
            assistant_model_id: config.assistant_model_id,
            name: config.name,
            value: config.value,
            value_type: config.value_type,
        });
    }
    println!("model_configs: {:?}", model_configs);

    // Model and prompt params are empty
    let model = vec![AssistantModel {
        id: model_id,
        assistant_id,
        provider_id: 0,
        model_code: "".to_string(),
        alias: "".to_string(),
    }];
    let prompt_params = Vec::new();

    // Build AssistantDetail object
    let assistant_detail = AssistantDetail {
        assistant,
        prompts,
        model,
        model_configs,
        prompt_params,
        mcp_configs: Vec::new(),
        mcp_tool_configs: Vec::new(),
    };

    Ok(assistant_detail)
}

#[tauri::command]
pub fn copy_assistant(
    app_handle: tauri::AppHandle,
    assistant_id: i64,
) -> Result<AssistantDetail, String> {
    println!("Start copying assistant with ID: {}", assistant_id);
    let assistant_db = AssistantDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    // Get the original assistant
    let original_assistant = assistant_db
        .get_assistant(assistant_id)
        .map_err(|e| e.to_string())?;

    // Create a new assistant based on the original
    let new_assistant_id = assistant_db
        .add_assistant(
            &format!("副本 {}", original_assistant.name),
            &original_assistant.description.unwrap(),
            original_assistant.assistant_type,
            original_assistant.is_addition,
        )
        .map_err(|e| e.to_string())?;

    // Copy prompts
    let original_prompts = assistant_db
        .get_assistant_prompt(assistant_id)
        .map_err(|e| e.to_string())?;
    let mut new_prompts = Vec::new();
    for prompt in original_prompts {
        let new_prompt_id = assistant_db
            .add_assistant_prompt(new_assistant_id, &prompt.prompt)
            .map_err(|e| e.to_string())?;
        new_prompts.push(AssistantPrompt {
            id: new_prompt_id,
            assistant_id: new_assistant_id,
            prompt: prompt.prompt,
            created_time: None,
        });
    }

    // Copy models and their configs
    let original_models = assistant_db
        .get_assistant_model(assistant_id)
        .map_err(|e| e.to_string())?;
    let mut new_models = Vec::new();
    let mut new_model_configs = Vec::new();
    for model in original_models {
        let new_model_id = assistant_db
            .add_assistant_model(
                new_assistant_id,
                model.provider_id,
                &model.model_code,
                &model.alias,
            )
            .map_err(|e| e.to_string())?;
        new_models.push(AssistantModel {
            id: new_model_id,
            assistant_id: new_assistant_id,
            provider_id: model.provider_id,
            model_code: model.model_code,
            alias: model.alias,
        });

        // Copy model configs
        let original_configs = assistant_db
            .get_assistant_model_configs_with_model_id(assistant_id, model.id)
            .map_err(|e| e.to_string())?;
        for config in original_configs {
            let new_config_id = assistant_db
                .add_assistant_model_config(
                    new_assistant_id,
                    new_model_id,
                    &config.name,
                    config.value.as_deref().unwrap_or(""),
                    &config.value_type,
                )
                .map_err(|e| e.to_string())?;
            new_model_configs.push(AssistantModelConfig {
                id: new_config_id,
                assistant_id: new_assistant_id,
                assistant_model_id: new_model_id,
                name: config.name,
                value: config.value,
                value_type: config.value_type,
            });
        }
    }

    // Get the newly created assistant
    let new_assistant = assistant_db
        .get_assistant(new_assistant_id)
        .map_err(|e| e.to_string())?;

    // Build AssistantDetail object
    let assistant_detail = AssistantDetail {
        assistant: new_assistant,
        prompts: new_prompts,
        model: new_models,
        model_configs: new_model_configs,
        prompt_params: Vec::new(), // Assuming prompt_params are not copied
        mcp_configs: Vec::new(),
        mcp_tool_configs: Vec::new(),
    };

    println!(
        "Successfully copied assistant. New assistant ID: {}",
        new_assistant_id
    );
    Ok(assistant_detail)
}

#[tauri::command]
pub fn delete_assistant(app_handle: tauri::AppHandle, assistant_id: i64) -> Result<(), String> {
    let assistant_db = AssistantDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    // 需要检查一下是不是快速使用助手，如果是，就不能够删除
    if assistant_id == 1 {
        return Err("快速使用助手不能删除".to_string());
    }

    let _ = assistant_db
        .delete_assistant_model_config_by_assistant_id(assistant_id)
        .map_err(|e| e.to_string());
    let _ = assistant_db
        .delete_assistant_prompt_by_assistant_id(assistant_id)
        .map_err(|e| e.to_string());
    let _ = assistant_db
        .delete_assistant_prompt_param_by_assistant_id(assistant_id)
        .map_err(|e| e.to_string());

    let conversation_db = ConversationDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let _ = conversation_db
        .conversation_repo()
        .unwrap()
        .update_assistant_id(assistant_id, Some(1))
        .map_err(|e| e.to_string())?;

    assistant_db
        .delete_assistant(assistant_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_assistant_field_value(
    app_handle: tauri::AppHandle,
    assistant_id: i64,
    field_name: &str,
) -> Result<String, String> {
    let assistant_db = AssistantDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    if field_name == "prompt" {
        // Get prompts for this assistant
        let prompts = assistant_db
            .get_assistant_prompt(assistant_id)
            .map_err(|e| e.to_string())?;

        println!("Prompts for assistant {}: {:?}", assistant_id, prompts);

        // Return first prompt's content
        return prompts
            .first()
            .map(|p| p.prompt.clone())
            .ok_or_else(|| "No prompt found".to_string());
    }

    // Get all model configs for this assistant
    let configs = assistant_db
        .get_assistant_model_configs(assistant_id)
        .map_err(|e| e.to_string())?;

    println!("Configs for assistant {}: {:?}", assistant_id, configs);

    // Find config with matching name
    configs
        .iter()
        .find(|config| config.name == field_name)
        .and_then(|config| config.value.clone())
        .ok_or_else(|| format!("Field '{}' not found", field_name))
}

// MCP Configuration Commands

#[tauri::command]
pub async fn update_assistant_mcp_config(
    app_handle: tauri::AppHandle,
    assistant_id: i64,
    mcp_server_id: i64,
    is_enabled: bool,
) -> Result<(), String> {
    let assistant_db = AssistantDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    assistant_db
        .upsert_assistant_mcp_config(assistant_id, mcp_server_id, is_enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_assistant_mcp_tool_config(
    app_handle: tauri::AppHandle,
    assistant_id: i64,
    mcp_tool_id: i64,
    is_enabled: bool,
    is_auto_run: bool,
) -> Result<(), String> {
    let assistant_db = AssistantDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    assistant_db
        .upsert_assistant_mcp_tool_config(assistant_id, mcp_tool_id, is_enabled, is_auto_run)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_assistant_mcp_servers_with_tools(
    app_handle: tauri::AppHandle,
    assistant_id: i64,
) -> Result<Vec<MCPServerWithTools>, String> {
    let assistant_db = AssistantDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let servers_data = assistant_db
        .get_assistant_mcp_servers_with_tools(assistant_id)
        .map_err(|e| e.to_string())?;

    let servers = servers_data
        .into_iter()
        .map(|(server_id, server_name, server_is_enabled, tools_data)| {
            let tools = tools_data
                .into_iter()
                .map(
                    |(tool_id, tool_name, tool_is_enabled, tool_is_auto_run)| MCPToolInfo {
                        id: tool_id,
                        name: tool_name,
                        is_enabled: tool_is_enabled,
                        is_auto_run: tool_is_auto_run,
                    },
                )
                .collect();

            MCPServerWithTools {
                id: server_id,
                name: server_name,
                is_enabled: server_is_enabled,
                tools,
            }
        })
        .collect();

    Ok(servers)
}

#[tauri::command]
pub async fn bulk_update_assistant_mcp_tools(
    app_handle: tauri::AppHandle,
    assistant_id: i64,
    mcp_server_id: i64,
    is_enabled: bool,
    is_auto_run: Option<bool>,
) -> Result<(), String> {
    let assistant_db = AssistantDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    // Get all tools for this server from the optimized method
    let servers_data = assistant_db
        .get_assistant_mcp_servers_with_tools(assistant_id)
        .map_err(|e| e.to_string())?;

    // Find the specific server and get its tools
    let tools_data = servers_data
        .into_iter()
        .find(|(server_id, _, _, _)| *server_id == mcp_server_id)
        .map(|(_, _, _, tools)| tools)
        .unwrap_or_default();

    // Update each tool
    for (tool_id, _, _, current_auto_run) in tools_data {
        let auto_run = is_auto_run.unwrap_or(current_auto_run);
        assistant_db
            .upsert_assistant_mcp_tool_config(assistant_id, tool_id, is_enabled, auto_run)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn update_assistant_model_config_value(
    app_handle: tauri::AppHandle,
    assistant_id: i64,
    config_name: String,
    config_value: String,
    value_type: String,
) -> Result<(), String> {
    let assistant_db = AssistantDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    
    // 首先尝试查找是否已存在该配置
    let existing_configs = assistant_db
        .get_assistant_model_configs(assistant_id)
        .map_err(|e| e.to_string())?;
    
    if let Some(existing_config) = existing_configs.iter().find(|c| c.name == config_name) {
        // 更新现有配置
        assistant_db
            .update_assistant_model_config(existing_config.id, &config_name, &config_value)
            .map_err(|e| e.to_string())?;
    } else {
        // 创建新配置 - 需要获取assistant_model_id
        let models = assistant_db
            .get_assistant_model(assistant_id)
            .map_err(|e| e.to_string())?;
        
        let model_id = if let Some(model) = models.first() {
            model.id
        } else {
            // 如果没有模型，创建一个默认模型
            assistant_db
                .add_assistant_model(assistant_id, 0, "", "")
                .map_err(|e| e.to_string())?
        };
        
        assistant_db
            .add_assistant_model_config(
                assistant_id,
                model_id,
                &config_name,
                &config_value,
                &value_type,
            )
            .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}
