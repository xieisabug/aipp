#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod api;
mod mcp;
mod artifacts;
mod db;
mod errors;
mod plugin;
mod state;
mod template_engine;
mod utils;
mod window;

use crate::api::ai_api::{
    ask_ai, cancel_ai, regenerate_ai, regenerate_conversation_title, tool_result_continue_ask_ai,
};
use crate::artifacts::env_installer::{check_bun_version, check_uv_version, install_bun, install_uv};
use crate::artifacts::preview_router::{confirm_environment_install, preview_react_component, retry_preview_after_install, run_artifacts};
use crate::artifacts::collection_api::{
    delete_artifact_collection, generate_artifact_metadata, get_artifact_by_id,
    get_artifacts_collection, get_artifacts_for_completion, get_artifacts_statistics,
    open_artifact_window, save_artifact_to_collection, search_artifacts_collection,
    update_artifact_collection,
};
use crate::api::assistant_api::{
    add_assistant, bulk_update_assistant_mcp_tools, copy_assistant, delete_assistant,
    export_assistant, get_assistant, get_assistant_field_value,
    get_assistant_mcp_servers_with_tools, get_assistants, import_assistant, save_assistant,
    update_assistant_mcp_config, update_assistant_mcp_tool_config,
    update_assistant_model_config_value,
};
use crate::api::attachment_api::{add_attachment, open_attachment_with_default_app};
use crate::api::conversation_api::{
    create_conversation_with_messages, create_message, delete_conversation, fork_conversation, get_conversation_with_messages, list_conversations,
    update_assistant_message, update_conversation, update_message_content,
};
use crate::api::llm_api::{
    add_llm_model, add_llm_provider, delete_llm_model, delete_llm_provider, export_llm_provider,
    fetch_model_list, get_llm_models, get_llm_provider_config, get_llm_providers,
    get_models_for_select, import_llm_provider, preview_model_list, update_llm_provider,
    update_llm_provider_config, update_selected_models,
};
use crate::mcp::registry_api::{
    add_mcp_server, build_mcp_prompt, delete_mcp_server, get_mcp_provider,
    get_mcp_server, get_mcp_server_prompts, get_mcp_server_resources, get_mcp_server_tools,
    get_mcp_servers, refresh_mcp_server_capabilities, test_mcp_connection, toggle_mcp_server,
    update_mcp_server, update_mcp_server_prompt, update_mcp_server_tool,
};
use crate::mcp::builtin_mcp::{
    list_aipp_builtin_templates, add_or_update_aipp_builtin_server, execute_aipp_builtin_tool,
};
use crate::mcp::execution_api::{
    create_mcp_tool_call, execute_mcp_tool_call, get_mcp_tool_call,
    get_mcp_tool_calls_by_conversation,
};
use crate::api::system_api::{
    get_all_feature_config, get_bang_list, get_selected_text_api, open_data_folder,
    save_feature_config,
};
use crate::api::sub_task_api::{
    cancel_sub_task_execution, cancel_sub_task_execution_for_ui, create_sub_task_execution, delete_sub_task_definition,
    get_sub_task_definition, get_sub_task_execution_detail, get_sub_task_execution_detail_for_ui,
    list_sub_task_definitions, list_sub_task_executions, register_sub_task_definition, 
    run_sub_task_sync, run_sub_task_with_mcp_loop, sub_task_regist, update_sub_task_definition,
};
use crate::artifacts::react_preview::{
    close_react_preview, create_react_preview, create_react_preview_for_artifact,
};
use crate::artifacts::vue_preview::{
    close_vue_preview, create_vue_preview, create_vue_preview_for_artifact,
};
use crate::artifacts::{
    react_runner::{close_react_artifact, run_react_artifact},
    vue_runner::{close_vue_artifact, run_vue_artifact},
};
use crate::artifacts::artifacts_db::ArtifactsDatabase;
use crate::db::assistant_db::AssistantDatabase;
use crate::db::llm_db::LLMDatabase;
use crate::db::sub_task_db::SubTaskDatabase;
use crate::mcp::mcp_db::MCPDatabase;
use crate::db::system_db::SystemDatabase;
use crate::window::{
    awaken_aipp, create_ask_window, handle_open_ask_window,
    open_artifact_collections_window, open_artifact_preview_window, open_chat_ui_window,
    open_config_window, open_plugin_store_window, open_plugin_window, ensure_hidden_search_window,
};
use chrono::Local;
use db::conversation_db::ConversationDatabase;
use db::database_upgrade;
use db::plugin_db::PluginDatabase;
use db::system_db::FeatureConfig;
use get_selected_text::get_selected_text;
use serde::{Deserialize, Serialize};
use state::message_token::MessageTokenManager;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::path::BaseDirectory;
use tauri::Emitter;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    Manager, RunEvent,
};
use tokio::sync::Mutex as TokioMutex;

struct AppState {
    selected_text: TokioMutex<String>,
}

#[derive(Clone)]
struct FeatureConfigState {
    configs: Arc<TokioMutex<Vec<FeatureConfig>>>,
    config_feature_map: Arc<TokioMutex<HashMap<String, HashMap<String, FeatureConfig>>>>,
}

#[derive(Clone)]
struct NameCacheState {
    assistant_names: Arc<TokioMutex<HashMap<i64, String>>>,
    model_names: Arc<TokioMutex<HashMap<i64, String>>>,
}

#[derive(Serialize, Deserialize)]
struct Config {
    selected_text: String,
}

#[cfg(target_os = "macos")]
fn query_accessibility_permissions() -> bool {
    let trusted = macos_accessibility_client::accessibility::application_is_trusted();
    if trusted {
        print!("Application is totally trusted!");
    } else {
        print!("Application isn't trusted :(");
        // let trusted = macos_accessibility_client::accessibility::application_is_trusted_with_prompt();
        // return trusted;
    }
    trusted
}

#[cfg(not(target_os = "macos"))]
fn query_accessibility_permissions() -> bool {
    return true;
}

#[tauri::command]
async fn get_selected() -> Result<String, String> {
    let result = get_selected_text().unwrap_or_default();
    println!("{:?}", result);
    Ok(result)
}

#[tauri::command]
async fn save_config(state: tauri::State<'_, AppState>, config: Config) -> Result<(), String> {
    let mut selected_text = state.selected_text.lock().await;
    *selected_text = config.selected_text;
    Ok(())
}

#[tauri::command]
async fn get_config(state: tauri::State<'_, AppState>) -> Result<Config, String> {
    let selected_text = state.selected_text.lock().await;
    Ok(Config { selected_text: selected_text.clone() })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle = app.handle();

            // 系统托盘菜单和图标初始化
            let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            let show = MenuItemBuilder::with_id("show", "显示").build(app)?;
            let tray_menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

            let tray = app.tray_by_id("aipp").unwrap();
            tray.set_menu(Some(tray_menu))?;
            tray.on_menu_event(move |app, event| match event.id().as_ref() {
                "quit" => {
                    std::process::exit(0);
                }
                "show" => {
                    awaken_aipp(&app);
                }
                _ => {}
            });
            let _ = tray.set_show_menu_on_left_click(true);

            if !query_accessibility_permissions() {
                println!("Please grant accessibility permissions to the app");
            } else {
                // 注册全局快捷键
                #[cfg(desktop)]
                {
                    register_global_shortcuts(&app_handle);
                }
            }

            let resource_path = app.path().resolve(
                "artifacts/templates/react/PreviewReactWindow.tsx",
                BaseDirectory::Resource,
            )?;
            println!("resource_path: {:?}", resource_path);

            let system_db = SystemDatabase::new(&app_handle)?;
            let llm_db = LLMDatabase::new(&app_handle)?;
            let assistant_db = AssistantDatabase::new(&app_handle)?;
            let conversation_db = ConversationDatabase::new(&app_handle)?;
            let plugin_db = PluginDatabase::new(&app_handle)?;
            let mcp_db = MCPDatabase::new(&app_handle)?;
            let sub_task_db = SubTaskDatabase::new(&app_handle)?;
            let artifacts_db = ArtifactsDatabase::new(&app_handle)?;

            system_db.create_tables()?;
            llm_db.create_tables()?;
            assistant_db.create_tables()?;
            conversation_db.create_tables()?;
            plugin_db.create_tables()?;
            mcp_db.create_tables()?;
            sub_task_db.create_tables()?;
            artifacts_db.create_tables()?;

            let _ = database_upgrade(&app_handle, system_db, llm_db, assistant_db, conversation_db);

            // 无需启动时初始化内置服务器，改为使用模板创建

            app.manage(initialize_state(&app_handle));
            app.manage(initialize_name_cache_state(&app_handle));

            if app.get_webview_window("main").is_none() {
                create_ask_window(&app_handle)
            }

            Ok(())
        })
        .manage(AppState { selected_text: TokioMutex::new(String::new()) })
        .manage(MessageTokenManager::new())
        .invoke_handler(tauri::generate_handler![
            ask_ai,
            tool_result_continue_ask_ai,
            regenerate_ai,
            regenerate_conversation_title,
            generate_artifact_metadata,
            cancel_ai,
            get_selected,
            open_config_window,
            open_chat_ui_window,
            open_plugin_window,
            open_plugin_store_window,
            open_artifact_preview_window,
            save_config,
            get_config,
            get_all_feature_config,
            save_feature_config,
            open_data_folder,
            get_llm_providers,
            update_llm_provider,
            add_llm_provider,
            delete_llm_provider,
            get_llm_provider_config,
            update_llm_provider_config,
            get_llm_models,
            fetch_model_list,
            preview_model_list,
            update_selected_models,
            get_models_for_select,
            add_llm_model,
            delete_llm_model,
            export_llm_provider,
            import_llm_provider,
            add_attachment,
            open_attachment_with_default_app,
            get_assistants,
            get_assistant,
            get_assistant_field_value,
            save_assistant,
            add_assistant,
            delete_assistant,
            copy_assistant,
            export_assistant,
            import_assistant,
            list_conversations,
            get_conversation_with_messages,
            create_conversation_with_messages,
            delete_conversation,
            fork_conversation,
            update_conversation,
            update_message_content,
            run_artifacts,
            save_artifact_to_collection,
            get_artifacts_collection,
            get_artifact_by_id,
            search_artifacts_collection,
            update_artifact_collection,
            delete_artifact_collection,
            open_artifact_window,
            open_artifact_collections_window,
            get_artifacts_statistics,
            get_artifacts_for_completion,
            get_bang_list,
            get_selected_text_api,
            check_bun_version,
            check_uv_version,
            install_bun,
            install_uv,
            preview_react_component,
            create_react_preview,
            create_react_preview_for_artifact,
            close_react_preview,
            create_vue_preview,
            create_vue_preview_for_artifact,
            close_vue_preview,
            run_react_artifact,
            close_react_artifact,
            run_vue_artifact,
            close_vue_artifact,
            confirm_environment_install,
            retry_preview_after_install,
            get_mcp_servers,
            get_mcp_server,
            get_mcp_provider,
            build_mcp_prompt,
            create_message,
            update_assistant_message,
            add_mcp_server,
            update_mcp_server,
            delete_mcp_server,
            toggle_mcp_server,
            get_mcp_server_tools,
            update_mcp_server_tool,
            get_mcp_server_resources,
            get_mcp_server_prompts,
            update_mcp_server_prompt,
            test_mcp_connection,
            refresh_mcp_server_capabilities,
            get_assistant_mcp_servers_with_tools,
            update_assistant_mcp_config,
            update_assistant_mcp_tool_config,
            bulk_update_assistant_mcp_tools,
            update_assistant_model_config_value,
            create_mcp_tool_call,
            execute_mcp_tool_call,
            get_mcp_tool_call,
            get_mcp_tool_calls_by_conversation,
            list_aipp_builtin_templates,
            add_or_update_aipp_builtin_server,
            execute_aipp_builtin_tool,
            register_sub_task_definition,
            run_sub_task_sync,
            run_sub_task_with_mcp_loop,
            sub_task_regist,
            list_sub_task_definitions,
            get_sub_task_definition,
            update_sub_task_definition,
            delete_sub_task_definition,
            create_sub_task_execution,
            list_sub_task_executions,
            get_sub_task_execution_detail,
            get_sub_task_execution_detail_for_ui,
            cancel_sub_task_execution,
            cancel_sub_task_execution_for_ui,
            ensure_hidden_search_window
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|_app_handle, e| match e {
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => {
            awaken_aipp(_app_handle);
        }
        _ => {}
    });

    Ok(())
}

fn initialize_state(app_handle: &tauri::AppHandle) -> FeatureConfigState {
    let db = SystemDatabase::new(app_handle).expect("Failed to connect to database");
    let configs = db.get_all_feature_config().expect("Failed to load feature configs");
    let mut configs_map = HashMap::new();
    for config in configs.clone().into_iter() {
        let feature_code = config.feature_code.clone();
        let key = config.key.clone();
        configs_map
            .entry(feature_code.clone())
            .or_insert(HashMap::new())
            .insert(key.clone(), config);
    }
    FeatureConfigState {
        configs: Arc::new(TokioMutex::new(configs)),
        config_feature_map: Arc::new(TokioMutex::new(configs_map)),
    }
}

fn initialize_name_cache_state(app_handle: &tauri::AppHandle) -> NameCacheState {
    let assistant_db = AssistantDatabase::new(app_handle).expect("Failed to connect to database");
    let assistants = assistant_db.get_assistants().expect("Failed to load assistants");
    let mut assistant_names = HashMap::new();
    for assistant in assistants.clone().into_iter() {
        assistant_names.insert(assistant.id, assistant.name.clone());
    }

    let llm_db = LLMDatabase::new(app_handle).expect("Failed to connect to database");
    let models = llm_db.get_models_for_select().expect("Failed to load models");
    let mut model_names = HashMap::new();
    for model in models.clone().into_iter() {
        model_names.insert(model.2, model.0);
    }

    NameCacheState {
        assistant_names: Arc::new(TokioMutex::new(assistant_names)),
        model_names: Arc::new(TokioMutex::new(model_names)),
    }
}

#[cfg(desktop)]
fn register_global_shortcuts(app_handle: &tauri::AppHandle) {
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};
    
    println!("开始注册全局快捷键...");
    
    // 创建快捷键定义
    let ctrl_shift_i_shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyI);
    let ctrl_shift_o_shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyO);
    
    // 首先尝试注册所有快捷键
    let all_shortcuts = [ctrl_shift_i_shortcut.clone(), ctrl_shift_o_shortcut.clone()];
    
    let builder_result = tauri_plugin_global_shortcut::Builder::new()
        .with_shortcuts(all_shortcuts);
        
    match builder_result {
        Ok(builder) => {
            match app_handle.plugin(
                builder.with_handler({
                    let ctrl_shift_i = ctrl_shift_i_shortcut.clone();
                    let ctrl_shift_o = ctrl_shift_o_shortcut.clone();
                    move |_app, shortcut, event| {
                        println!("{:?}", shortcut);
                        if shortcut == &ctrl_shift_i {
                            match event.state() {
                                ShortcutState::Pressed => {
                                    println!("CmdOrCtrl+Shift+I Pressed!");
                                }
                                ShortcutState::Released => {
                                    println!(
                                        "CmdOrCtrl+Shift+I pressed at time : {}",
                                        &Local::now().to_string()
                                    );
                                    match get_selected_text() {
                                        Ok(selected_text) => {
                                            println!(
                                                "Selected text: {}, at time: {}",
                                                selected_text.clone(),
                                                &Local::now().to_string()
                                            );
                                            let _ = _app.emit(
                                                "get_selected_text_event",
                                                selected_text.clone(),
                                            );
                                            let app_state = _app.try_state::<AppState>();
                                            if let Some(state) = app_state {
                                                *state.selected_text.blocking_lock() = selected_text;
                                            }
                                        }
                                        Err(e) => {
                                            println!("Error getting selected text: {}", e);
                                        }
                                    }
                                    handle_open_ask_window(_app);
                                }
                            }
                        } else if shortcut == &ctrl_shift_o {
                            match event.state() {
                                ShortcutState::Pressed => {
                                    println!("CmdOrCtrl+Shift+O Pressed!");
                                }
                                ShortcutState::Released => {
                                    println!(
                                        "CmdOrCtrl+Shift+O pressed at time : {}",
                                        &Local::now().to_string()
                                    );
                                    handle_open_ask_window(_app);
                                }
                            }
                        }
                    }
                }).build(),
            ) {
                Ok(_) => {
                    println!("✓ 成功注册所有全局快捷键: Ctrl+Shift+I, Ctrl+Shift+O");
                }
                Err(e) => {
                    println!("⚠ 无法注册全局快捷键 (可能已被其他应用占用): {}", e);
                    println!("尝试单独注册每个快捷键...");
                    
                    // 逐个尝试注册快捷键
                    register_individual_shortcuts(app_handle, ctrl_shift_i_shortcut, ctrl_shift_o_shortcut);
                }
            }
        }
        Err(e) => {
            println!("⚠ 创建快捷键构建器时出错: {}", e);
            println!("尝试单独注册每个快捷键...");
            register_individual_shortcuts(app_handle, ctrl_shift_i_shortcut, ctrl_shift_o_shortcut);
        }
    }
}

#[cfg(desktop)]
fn register_individual_shortcuts(
    app_handle: &tauri::AppHandle,
    ctrl_shift_i_shortcut: tauri_plugin_global_shortcut::Shortcut,
    ctrl_shift_o_shortcut: tauri_plugin_global_shortcut::Shortcut,
) {
    use tauri_plugin_global_shortcut::{ShortcutState};
    
    let mut registered_count = 0;
    
    // 尝试注册 Ctrl+Shift+I
    let builder_result_i = tauri_plugin_global_shortcut::Builder::new()
        .with_shortcuts([ctrl_shift_i_shortcut.clone()]);
        
    if let Ok(builder) = builder_result_i {
        match app_handle.plugin(
            builder.with_handler({
                let ctrl_shift_i = ctrl_shift_i_shortcut.clone();
                move |_app, shortcut, event| {
                    if shortcut == &ctrl_shift_i && event.state() == ShortcutState::Released {
                        println!("CmdOrCtrl+Shift+I pressed at time : {}", &Local::now().to_string());
                        match get_selected_text() {
                            Ok(selected_text) => {
                                println!("Selected text: {}, at time: {}", selected_text.clone(), &Local::now().to_string());
                                let _ = _app.emit("get_selected_text_event", selected_text.clone());
                                let app_state = _app.try_state::<AppState>();
                                if let Some(state) = app_state {
                                    *state.selected_text.blocking_lock() = selected_text;
                                }
                            }
                            Err(e) => {
                                println!("Error getting selected text: {}", e);
                            }
                        }
                        handle_open_ask_window(_app);
                    }
                }
            }).build(),
        ) {
            Ok(_) => {
                println!("✓ 成功注册快捷键: Ctrl+Shift+I");
                registered_count += 1;
            }
            Err(e) => {
                println!("⚠ 无法注册快捷键 Ctrl+Shift+I: {}", e);
            }
        }
    } else {
        println!("⚠ 创建 Ctrl+Shift+I 快捷键构建器时出错");
    }
    
    // 尝试注册 Ctrl+Shift+O
    let builder_result_o = tauri_plugin_global_shortcut::Builder::new()
        .with_shortcuts([ctrl_shift_o_shortcut.clone()]);
        
    if let Ok(builder) = builder_result_o {
        match app_handle.plugin(
            builder.with_handler({
                let ctrl_shift_o = ctrl_shift_o_shortcut.clone();
                move |_app, shortcut, event| {
                    if shortcut == &ctrl_shift_o && event.state() == ShortcutState::Released {
                        println!("CmdOrCtrl+Shift+O pressed at time : {}", &Local::now().to_string());
                        handle_open_ask_window(_app);
                    }
                }
            }).build(),
        ) {
            Ok(_) => {
                println!("✓ 成功注册快捷键: Ctrl+Shift+O");
                registered_count += 1;
            }
            Err(e) => {
                println!("⚠ 无法注册快捷键 Ctrl+Shift+O: {}", e);
            }
        }
    } else {
        println!("⚠ 创建 Ctrl+Shift+O 快捷键构建器时出错");
    }
    
    if registered_count == 0 {
        println!("⚠ 所有全局快捷键都无法注册，但应用程序将继续正常运行");
        println!("  提示：快捷键冲突通常是由于其他应用程序已注册相同的快捷键组合");
    } else {
        println!("✓ 成功注册 {} 个全局快捷键", registered_count);
    }
}
