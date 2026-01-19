mod commands;
mod injector;
mod layout;

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder},
    webview::{NewWindowResponse, WebviewBuilder},
    LogicalPosition, LogicalSize, Manager, PhysicalSize, Position, Size, WebviewUrl, WindowEvent,
    TitleBarStyle,
};
use tauri_plugin_opener::OpenerExt;

const AI_SERVICES: [(&str, &str); 3] = [
    ("claude", "https://claude.ai/new"),
    ("chatgpt", "https://chat.openai.com/"),
    ("gemini", "https://gemini.google.com/app"),
];

const TITLEBAR_VIEW_PATH: &str = "index.html?view=titlebar";

// Default user agent (Safari)
const USER_AGENT_DEFAULT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15";

// Chrome user agent for Gemini (Google is more permissive with Chromium-based browsers)
const USER_AGENT_GEMINI: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";

// Gemini initialization script to bypass WebView detection
pub const GEMINI_INIT_SCRIPT: &str = r#"
(function() {
    if (window.__seno_patched) return;
    window.__seno_patched = true;

    // WebDriver detection bypass
    Object.defineProperty(navigator, 'webdriver', {
        get: () => undefined,
        configurable: true
    });

    // Chrome 91+ returns empty arrays for plugins and mimeTypes
    Object.defineProperty(navigator, 'plugins', {
        get: () => [],
        configurable: true
    });

    Object.defineProperty(navigator, 'mimeTypes', {
        get: () => [],
        configurable: true
    });

    Object.defineProperty(navigator, 'languages', {
        get: () => ['ja-JP', 'ja', 'en-US', 'en'],
        configurable: true
    });

    // Remove automation flags
    delete window.cdc_adoQpoasnfa76pfcZLmcfl_Array;
    delete window.cdc_adoQpoasnfa76pfcZLmcfl_Promise;
    delete window.cdc_adoQpoasnfa76pfcZLmcfl_Symbol;

    // MutationObserver to reapply patches after SPA navigation
    const observer = new MutationObserver(() => {
        if (navigator.webdriver !== undefined) {
            Object.defineProperty(navigator, 'webdriver', {
                get: () => undefined,
                configurable: true
            });
        }
    });
    observer.observe(document.documentElement, { childList: true, subtree: true });
})();
"#;

// Script for periodic re-injection (resets flag and reapplies patches)
pub const GEMINI_REINJECT_SCRIPT: &str = r#"
(function() {
    window.__seno_patched = false;

    Object.defineProperty(navigator, 'webdriver', {
        get: () => undefined,
        configurable: true
    });

    Object.defineProperty(navigator, 'plugins', {
        get: () => [],
        configurable: true
    });

    Object.defineProperty(navigator, 'mimeTypes', {
        get: () => [],
        configurable: true
    });

    Object.defineProperty(navigator, 'languages', {
        get: () => ['ja-JP', 'ja', 'en-US', 'en'],
        configurable: true
    });

    delete window.cdc_adoQpoasnfa76pfcZLmcfl_Array;
    delete window.cdc_adoQpoasnfa76pfcZLmcfl_Promise;
    delete window.cdc_adoQpoasnfa76pfcZLmcfl_Symbol;

    window.__seno_patched = true;
})();
"#;

fn get_user_agent(label: &str) -> &'static str {
    match label {
        "gemini" => USER_AGENT_GEMINI,
        _ => USER_AGENT_DEFAULT,
    }
}

// Fixed UUIDs for session persistence (as byte arrays)
const DATA_STORE_IDS: [(&str, [u8; 16]); 3] = [
    (
        "claude",
        [
            0xa1, 0xb2, 0xc3, 0xd4, 0xe5, 0xf6, 0x47, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45,
            0x67, 0x89,
        ],
    ),
    (
        "chatgpt",
        [
            0xb2, 0xc3, 0xd4, 0xe5, 0xf6, 0xa7, 0x48, 0x90, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56,
            0x78, 0x90,
        ],
    ),
    (
        "gemini",
        [
            0xc3, 0xd4, 0xe5, 0xf6, 0xa7, 0xb8, 0x49, 0x01, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67,
            0x89, 0x01,
        ],
    ),
];


fn get_data_store_id(label: &str) -> Option<[u8; 16]> {
    DATA_STORE_IDS
        .iter()
        .find(|(l, _)| *l == label)
        .map(|(_, id)| *id)
}

fn is_oauth_popup(url: &tauri::Url) -> bool {
    let url_str = url.as_str().to_lowercase();
    if url_str.is_empty() || url_str == "about:blank" || url_str == "about:srcdoc" {
        return true;
    }
    if url_str.contains("oauth")
        || url_str.contains("sso")
        || url_str.contains("signin")
        || url_str.contains("login")
        || url_str.contains("authorize")
    {
        return true;
    }

    const AUTH_HOST_SUFFIXES: [&str; 15] = [
        "accounts.google.com",
        "login.microsoftonline.com",
        "login.microsoft.com",
        "login.live.com",
        "appleid.apple.com",
        "id.apple.com",
        "auth0.com",
        "okta.com",
        "okta-emea.com",
        "okta-apac.com",
        "onelogin.com",
        "pingidentity.com",
        "pingone.com",
        "duosecurity.com",
        "duo.com",
    ];

    if let Some(host) = url.host_str() {
        let host = host.to_lowercase();
        if AUTH_HOST_SUFFIXES
            .iter()
            .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
        {
            return true;
        }
    }

    false
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::send_to_all,
            commands::reload_webview,
            commands::reload_all,
            commands::new_chat_all,
            commands::update_input_height,
            commands::zoom_in,
            commands::zoom_out,
            commands::zoom_reset,
            commands::clear_cache_all,
            commands::refresh_gemini_session,
        ])
        .setup(|app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;

            let app_handle = app.handle().clone();
            let main_window = app.get_webview_window("main").unwrap();
            let window = main_window.as_ref().window();
            let scale_factor = window.scale_factor()?;
            let physical_size = window.inner_size()?;

            let app_menu = SubmenuBuilder::new(app, "Seno")
                .item(&PredefinedMenuItem::about(app, None, None)?)
                .separator()
                .item(&PredefinedMenuItem::services(app, None)?)
                .separator()
                .item(&PredefinedMenuItem::hide(app, None)?)
                .item(&PredefinedMenuItem::hide_others(app, None)?)
                .item(&PredefinedMenuItem::show_all(app, None)?)
                .separator()
                .item(&PredefinedMenuItem::quit(app, None)?)
                .build()?;
            let view_menu = SubmenuBuilder::new(app, "View")
                .item(&MenuItemBuilder::with_id("zoom_in", "Zoom In")
                    .accelerator("CmdOrCtrl+Shift+=")
                    .build(app)?)
                .item(&MenuItemBuilder::with_id("zoom_in_alt", "Zoom In (Alt)")
                    .accelerator("CmdOrCtrl+=")
                    .build(app)?)
                .item(&MenuItemBuilder::with_id("zoom_out", "Zoom Out")
                    .accelerator("CmdOrCtrl+-")
                    .build(app)?)
                .item(&MenuItemBuilder::with_id("zoom_reset", "Actual Size")
                    .accelerator("CmdOrCtrl+0")
                    .build(app)?)
                .build()?;
            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .item(&PredefinedMenuItem::undo(app, None)?)
                .item(&PredefinedMenuItem::redo(app, None)?)
                .separator()
                .item(&PredefinedMenuItem::cut(app, None)?)
                .item(&PredefinedMenuItem::copy(app, None)?)
                .item(&PredefinedMenuItem::paste(app, None)?)
                .item(&PredefinedMenuItem::select_all(app, None)?)
                .build()?;
            let chat_menu = SubmenuBuilder::new(app, "Chat")
                .item(&MenuItemBuilder::with_id("new_chat_all", "New Chat (All)")
                    .accelerator("CmdOrCtrl+N")
                    .build(app)?)
                .item(&MenuItemBuilder::with_id("reload_all", "Reload All")
                    .accelerator("CmdOrCtrl+R")
                    .build(app)?)
                .separator()
                .item(&MenuItemBuilder::with_id("clear_cache", "Clear Cache")
                    .accelerator("CmdOrCtrl+Shift+Delete")
                    .build(app)?)
                .build()?;
            let menu = MenuBuilder::new(app)
                .items(&[&app_menu, &edit_menu, &view_menu, &chat_menu])
                .build()?;
            app.set_menu(menu)?;

            app_handle.clone().on_menu_event(move |_app_handle, event| {
                let app_handle = app_handle.clone();
                let id = event.id().0.clone();
                tauri::async_runtime::spawn(async move {
                    let result = match id.as_str() {
                        "zoom_in" => commands::zoom_in(app_handle).await.map(|_| ()),
                        "zoom_in_alt" => commands::zoom_in(app_handle).await.map(|_| ()),
                        "zoom_out" => commands::zoom_out(app_handle).await.map(|_| ()),
                        "zoom_reset" => commands::zoom_reset(app_handle).await.map(|_| ()),
                        "reload_all" => commands::reload_all(app_handle).await,
                        "new_chat_all" => commands::new_chat_all(app_handle).await,
                        "clear_cache" => commands::clear_cache_all(app_handle).await,
                        _ => Ok(()),
                    };

                    if let Err(error) = result {
                        eprintln!("Menu command failed: {error}");
                    }
                });
            });

            let app_handle = app.handle().clone();

            #[cfg(target_os = "macos")]
            {
                window.set_title_bar_style(TitleBarStyle::Overlay)?;
                window.set_title("")?;
            }

            let titlebar_builder =
                WebviewBuilder::new("titlebar", WebviewUrl::App(TITLEBAR_VIEW_PATH.into()))
                    .user_agent(USER_AGENT_DEFAULT);

            let _titlebar = window.add_child(
                titlebar_builder,
                Position::Logical(LogicalPosition { x: 0.0, y: 0.0 }),
                Size::Logical(LogicalSize {
                    width: 1.0,
                    height: 1.0,
                }),
            )?;

            // Add AI webviews as children of the main window
            for (label, url) in AI_SERVICES.iter() {
                let app_handle = app_handle.clone();
                let mut builder =
                    WebviewBuilder::new(*label, WebviewUrl::External(url.parse().unwrap()))
                        .user_agent(get_user_agent(label))
                        .on_new_window(move |url, _features| {
                            let url_str = url.to_string();
                            if is_oauth_popup(&url) {
                                // Allow OAuth popups inside the app to preserve session cookies.
                                return NewWindowResponse::Allow;
                            }

                            // Open target=_blank links in the default browser.
                            match app_handle.opener().open_url(url_str, None::<&str>) {
                                Ok(_) => NewWindowResponse::Deny,
                                Err(_) => NewWindowResponse::Allow,
                            }
                        });

                // Add initialization script for Gemini to bypass WebView detection
                if *label == "gemini" {
                    builder = builder.initialization_script(GEMINI_INIT_SCRIPT);
                }

                // Set data store identifier for session persistence (macOS)
                #[cfg(target_os = "macos")]
                if let Some(data_id) = get_data_store_id(label) {
                    builder = builder.data_store_identifier(data_id);
                }

                let _webview = window.add_child(
                    builder,
                    Position::Logical(LogicalPosition { x: 0.0, y: 0.0 }),
                    Size::Logical(LogicalSize {
                        width: 1.0,
                        height: 1.0,
                    }),
                )?;
            }

            let labels = ai_labels();
            layout::apply_layout(app.handle(), &labels, physical_size, scale_factor)
                .map_err(|e| e.to_string())?;

            // Show window after setup
            window.show()?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            let result = match event {
                WindowEvent::Resized(size) => {
                    if size.width > 0 && size.height > 0 {
                        apply_layout_for_window(window, *size, None)
                    } else {
                        Ok(())
                    }
                }
                WindowEvent::ScaleFactorChanged {
                    scale_factor,
                    new_inner_size,
                    ..
                } => {
                    if new_inner_size.width > 0 && new_inner_size.height > 0 {
                        apply_layout_for_window(window, *new_inner_size, Some(*scale_factor))
                    } else {
                        Ok(())
                    }
                }
                _ => Ok(()),
            };
            if let Err(error) = result {
                eprintln!("Failed to apply layout: {error}");
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn apply_layout_for_window(
    window: &tauri::Window,
    physical_size: PhysicalSize<u32>,
    scale_factor_override: Option<f64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let scale_factor = if let Some(scale_factor) = scale_factor_override {
        scale_factor
    } else {
        window.scale_factor()?
    };
    let labels = ai_labels();
    layout::apply_layout(window.app_handle(), &labels, physical_size, scale_factor)?;
    Ok(())
}

fn ai_labels() -> Vec<&'static str> {
    AI_SERVICES.iter().map(|(label, _)| *label).collect()
}
