use crate::{
    db::{
        assistant_db::{
            Assistant, AssistantDatabase, AssistantModel, AssistantModelConfig, AssistantPrompt,
            AssistantPromptParam,
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
    println!("Assistant name: {:?} (id: {})", assistant.name, assistant.id);

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

    // 构建 AssistantDetail 对象
    let assistant_detail = AssistantDetail {
        assistant,
        prompts,
        model,
        model_configs,
        prompt_params,
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
