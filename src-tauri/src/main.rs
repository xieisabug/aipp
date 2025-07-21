#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod api;
mod artifacts;
mod db;
mod errors;
mod plugin;
mod state;
mod template_engine;
mod utils;
mod window;

use crate::api::ai_api::{ask_ai, cancel_ai, regenerate_ai, regenerate_conversation_title};
use crate::api::artifacts_api::{
    check_bun_version, check_uv_version, install_bun, install_uv, run_artifacts, open_react_component_preview, preview_react_component, confirm_environment_install, retry_preview_after_install,
};
use crate::api::assistant_api::{
    add_assistant, copy_assistant, delete_assistant, get_assistant, get_assistant_field_value,
    get_assistants, save_assistant,
};
use crate::api::attachment_api::{add_attachment, open_attachment_with_default_app};
use crate::api::conversation_api::{
    delete_conversation, get_conversation_with_messages, list_conversations, update_conversation,
};
use crate::api::llm_api::{
    add_llm_model, add_llm_provider, delete_llm_model, delete_llm_provider, fetch_model_list,
    get_llm_models, get_llm_provider_config, get_llm_providers, get_models_for_select,
    update_llm_provider, update_llm_provider_config,
};
use crate::api::system_api::{
    get_all_feature_config, get_bang_list, get_selected_text_api, open_data_folder,
    save_feature_config,
};
use crate::artifacts::react_preview::{create_react_preview, create_react_preview_for_artifact, close_react_preview};
use crate::artifacts::vue_preview::{create_vue_preview, create_vue_preview_for_artifact, close_vue_preview};
use crate::db::assistant_db::AssistantDatabase;
use crate::db::llm_db::LLMDatabase;
use crate::db::system_db::SystemDatabase;
use crate::window::{
    create_ask_window, open_chat_ui_window, open_config_window, open_plugin_window, open_preview_frontend_window, open_artifact_preview_window,
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
    Ok(Config {
        selected_text: selected_text.clone(),
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = tauri::Builder::default()
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
                    app.exit(0);
                }
                "show" => {
                    handle_open_ask_window(&app);
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
                    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};
                    let ctrl_shift_i_shortcut =
                        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyI);
                    let ctrl_shift_o_shortcut =
                        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyO);

                    app.handle().plugin(
                        tauri_plugin_global_shortcut::Builder::new()
                            .with_shortcuts([ctrl_shift_i_shortcut, ctrl_shift_o_shortcut])?
                            .with_handler(move |_app, shortcut, event| {
                                println!("{:?}", shortcut);
                                if shortcut == &ctrl_shift_i_shortcut {
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
                                                    *app_state
                                                        .unwrap()
                                                        .selected_text
                                                        .blocking_lock() = selected_text;
                                                }
                                                Err(e) => {
                                                    println!("Error getting selected text: {}", e);
                                                }
                                            }
                                            handle_open_ask_window(_app);
                                        }
                                    }
                                } else if shortcut == &ctrl_shift_o_shortcut {
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
                            })
                            .build(),
                    )?;
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
            system_db.create_tables()?;
            llm_db.create_tables()?;
            assistant_db.create_tables()?;
            conversation_db.create_tables()?;
            plugin_db.create_tables()?;

            let _ = database_upgrade(
                &app_handle,
                system_db,
                llm_db,
                assistant_db,
                conversation_db,
            );

            app.manage(initialize_state(&app_handle));
            app.manage(initialize_name_cache_state(&app_handle));

            if app.get_webview_window("main").is_none() {
                create_ask_window(&app_handle)
            }

            Ok(())
        })
        .manage(AppState {
            selected_text: TokioMutex::new(String::new()),
        })
        .manage(MessageTokenManager::new())
        .invoke_handler(tauri::generate_handler![
            ask_ai,
            regenerate_ai,
            regenerate_conversation_title,
            cancel_ai,
            get_selected,
            open_config_window,
            open_chat_ui_window,
            open_plugin_window,
            open_preview_frontend_window,
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
            get_models_for_select,
            add_llm_model,
            delete_llm_model,
            add_attachment,
            open_attachment_with_default_app,
            get_assistants,
            get_assistant,
            get_assistant_field_value,
            save_assistant,
            add_assistant,
            delete_assistant,
            copy_assistant,
            list_conversations,
            get_conversation_with_messages,
            delete_conversation,
            update_conversation,
            run_artifacts,
            get_bang_list,
            get_selected_text_api,
            check_bun_version,
            check_uv_version,
            install_bun,
            install_uv,
            open_react_component_preview,
            preview_react_component,
            create_react_preview,
            create_react_preview_for_artifact,
            close_react_preview,
            create_vue_preview,
            create_vue_preview_for_artifact,
            close_vue_preview,
            confirm_environment_install,
            retry_preview_after_install
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|_, e| match e {
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        _ => {}
    });

    Ok(())
}

fn handle_open_ask_window(app_handle: &tauri::AppHandle) {
    let ask_window = app_handle.get_webview_window("ask");
    let chat_ui_window = app_handle.get_webview_window("chat_ui");

    match (ask_window, chat_ui_window) {
        (None, _) => {
            println!(
                "Creating ask window, at time: {}",
                &Local::now().to_string()
            );
            create_ask_window(app_handle);
        }
        (Some(window), _) => {
            println!(
                "Focusing ask window, at time: {}",
                &Local::now().to_string()
            );
            if window.is_minimized().unwrap_or(false) {
                window.unminimize().unwrap();
            }
            window.show().unwrap();
            window.set_focus().unwrap();
        }
    }
}

fn initialize_state(app_handle: &tauri::AppHandle) -> FeatureConfigState {
    let db = SystemDatabase::new(app_handle).expect("Failed to connect to database");
    let configs = db
        .get_all_feature_config()
        .expect("Failed to load feature configs");
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
    let assistants = assistant_db
        .get_assistants()
        .expect("Failed to load assistants");
    let mut assistant_names = HashMap::new();
    for assistant in assistants.clone().into_iter() {
        assistant_names.insert(assistant.id, assistant.name.clone());
    }

    let llm_db = LLMDatabase::new(app_handle).expect("Failed to connect to database");
    let models = llm_db
        .get_models_for_select()
        .expect("Failed to load models");
    let mut model_names = HashMap::new();
    for model in models.clone().into_iter() {
        model_names.insert(model.2, model.0);
    }

    NameCacheState {
        assistant_names: Arc::new(TokioMutex::new(assistant_names)),
        model_names: Arc::new(TokioMutex::new(model_names)),
    }
}
