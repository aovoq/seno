use std::sync::atomic::{AtomicU32, Ordering};
use tauri::Manager;
use sysinfo::{Pid, System};

#[cfg(target_os = "macos")]
use std::collections::HashSet;
#[cfg(target_os = "macos")]
use std::path::Path;
#[cfg(target_os = "macos")]
use libproc::processes;

use crate::{injector, layout, GEMINI_REINJECT_SCRIPT};

const AI_SERVICES: [&str; 3] = ["claude", "chatgpt", "gemini"];

// Store zoom level as percentage (100 = 1.0x, 150 = 1.5x, etc.)
// Default is 100%
static ZOOM_LEVEL: AtomicU32 = AtomicU32::new(100);

#[tauri::command]
pub async fn send_to_all(app: tauri::AppHandle, text: String) -> Result<(), String> {
    let handles = AI_SERVICES
        .iter()
        .map(|label| {
            let app = app.clone();
            let label = label.to_string();
            let text = text.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(webview) = app.get_webview(&label) {
                    let script = injector::get_send_script(&label, &text);
                    webview.eval(&script).map_err(|e| e.to_string())?;
                }
                Ok::<(), String>(())
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.await.map_err(|e| e.to_string())??;
    }

    Ok(())
}

#[tauri::command]
pub async fn reload_webview(app: tauri::AppHandle, label: String) -> Result<(), String> {
    if let Some(webview) = app.get_webview(&label) {
        webview
            .eval("window.location.reload()")
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn reload_all(app: tauri::AppHandle) -> Result<(), String> {
    let handles = AI_SERVICES
        .iter()
        .map(|label| {
            let app = app.clone();
            let label = label.to_string();
            tauri::async_runtime::spawn(async move {
                if let Some(webview) = app.get_webview(&label) {
                    webview
                        .eval("window.location.reload()")
                        .map_err(|e| e.to_string())?;
                }
                Ok::<(), String>(())
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.await.map_err(|e| e.to_string())??;
    }

    Ok(())
}

#[tauri::command]
pub async fn new_chat_all(app: tauri::AppHandle) -> Result<(), String> {
    let script = injector::get_new_chat_script();
    let handles = AI_SERVICES
        .iter()
        .map(|label| {
            let app = app.clone();
            let label = label.to_string();
            tauri::async_runtime::spawn(async move {
                if let Some(webview) = app.get_webview(&label) {
                    webview.eval(script).map_err(|e| e.to_string())?;
                }
                Ok::<(), String>(())
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.await.map_err(|e| e.to_string())??;
    }

    // Delay to let SPA navigation complete before restoring focus
    tauri::async_runtime::spawn(async move {
        std::thread::sleep(std::time::Duration::from_millis(300));
        let _ = focus_input(app).await;
    });

    Ok(())
}

#[tauri::command]
pub async fn update_input_height(app: tauri::AppHandle, height: f64) -> Result<(), String> {
    let main_window = app.get_window("main").ok_or("Main window not found")?;
    let scale_factor = main_window.scale_factor().map_err(|e| e.to_string())?;
    let physical_size = main_window.inner_size().map_err(|e| e.to_string())?;

    #[cfg(debug_assertions)]
    eprintln!("update_input_height: requested={height}");

    layout::set_input_bar_height(height);

    let app_handle = app.clone();
    let labels = AI_SERVICES.iter().map(|label| label.to_string()).collect::<Vec<_>>();

    main_window
        .run_on_main_thread(move || {
            let label_refs = labels.iter().map(|label| label.as_str()).collect::<Vec<_>>();
            if let Err(error) =
                layout::apply_layout(&app_handle, &label_refs, physical_size, scale_factor)
            {
                eprintln!("Failed to apply layout: {error}");
            } else {
                #[cfg(debug_assertions)]
                eprintln!("layout applied");
            }
        })
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn zoom_in(app: tauri::AppHandle) -> Result<f64, String> {
    let current = ZOOM_LEVEL.load(Ordering::SeqCst);
    // Max zoom: 200%
    let new_level = (current + 10).min(200);
    ZOOM_LEVEL.store(new_level, Ordering::SeqCst);

    apply_zoom(&app, new_level).await?;
    Ok(new_level as f64 / 100.0)
}

#[tauri::command]
pub async fn zoom_out(app: tauri::AppHandle) -> Result<f64, String> {
    let current = ZOOM_LEVEL.load(Ordering::SeqCst);
    // Min zoom: 50%
    let new_level = (current.saturating_sub(10)).max(50);
    ZOOM_LEVEL.store(new_level, Ordering::SeqCst);

    apply_zoom(&app, new_level).await?;
    Ok(new_level as f64 / 100.0)
}

#[tauri::command]
pub async fn zoom_reset(app: tauri::AppHandle) -> Result<f64, String> {
    ZOOM_LEVEL.store(100, Ordering::SeqCst);
    apply_zoom(&app, 100).await?;
    Ok(1.0)
}

async fn apply_zoom(app: &tauri::AppHandle, level: u32) -> Result<(), String> {
    let zoom_factor = level as f64 / 100.0;
    let handles = AI_SERVICES
        .iter()
        .map(|label| {
            let app = app.clone();
            let label = label.to_string();
            tauri::async_runtime::spawn(async move {
                if let Some(webview) = app.get_webview(&label) {
                    webview.set_zoom(zoom_factor).map_err(|e| e.to_string())?;
                }
                Ok::<(), String>(())
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.await.map_err(|e| e.to_string())??;
    }

    Ok(())
}

#[tauri::command]
pub async fn clear_cache_all(app: tauri::AppHandle) -> Result<(), String> {
    let clear_script = r#"
        (async () => {
            try {
                localStorage.clear();
                sessionStorage.clear();
                if ('caches' in window) {
                    const names = await caches.keys();
                    await Promise.all(names.map(name => caches.delete(name)));
                }
            } catch (e) {
                console.error('Cache clear error:', e);
            }
        })();
    "#;

    let handles = AI_SERVICES
        .iter()
        .map(|label| {
            let app = app.clone();
            let label = label.to_string();
            tauri::async_runtime::spawn(async move {
                if let Some(webview) = app.get_webview(&label) {
                    webview.eval(clear_script).map_err(|e| e.to_string())?;
                }
                Ok::<(), String>(())
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.await.map_err(|e| e.to_string())??;
    }

    Ok(())
}

#[tauri::command]
pub async fn refresh_gemini_session(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(webview) = app.get_webview("gemini") {
        webview
            .eval(GEMINI_REINJECT_SCRIPT)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn focus_input(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(webview) = app.get_webview("main") {
        webview.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_memory_usage() -> Result<f64, String> {
    let pid = Pid::from_u32(std::process::id());
    let mut system = System::new_all();
    system.refresh_processes();

    let process = system
        .process(pid)
        .ok_or_else(|| "Process not found".to_string())?;

    let mut total_memory = process.memory();

    #[cfg(target_os = "macos")]
    {
        let user_id = process.user_id().cloned();
        let related_pids = macos_related_pids(&system, pid, process.start_time(), user_id);
        total_memory += sum_memory_for_pids(&system, related_pids);
    }

    #[cfg(not(target_os = "macos"))]
    {
        let descendant_pids = system
            .processes()
            .values()
            .filter(|proc| is_descendant(proc, pid, system.processes()))
            .map(|proc| proc.pid());
        total_memory += sum_memory_for_pids(&system, descendant_pids);
    }

    let memory_mb = total_memory as f64 / (1024.0 * 1024.0);
    Ok(memory_mb)
}

#[cfg(target_os = "macos")]
fn macos_webkit_data_store_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let bundle_id = "com.seno.viewer";
    let path = Path::new(&home).join("Library").join("WebKit").join(bundle_id);
    path.exists().then_some(path)
}

#[cfg(target_os = "macos")]
fn macos_related_pids(
    system: &System,
    root_pid: Pid,
    start_time: u64,
    user_id: Option<sysinfo::Uid>,
) -> HashSet<Pid> {
    let mut related_pids = HashSet::new();

    for proc in system.processes().values() {
        if is_descendant(proc, root_pid, system.processes()) {
            related_pids.insert(proc.pid());
        }
        if proc.name().starts_with("seno ") {
            related_pids.insert(proc.pid());
        }
    }

    if let Some(data_store_path) = macos_webkit_data_store_path() {
        if let Ok(pids) = processes::pids_by_path(&data_store_path, false, false) {
            for web_pid in pids {
                related_pids.insert(Pid::from_u32(web_pid));
            }
        }
    }

    for proc in system.processes().values() {
        if proc.name() != "com.apple.WebKit.WebContent" {
            continue;
        }
        if proc.start_time() < start_time {
            continue;
        }
        if let Some(user_id) = user_id.as_ref() {
            if proc.user_id() != Some(user_id) {
                continue;
            }
        }
        related_pids.insert(proc.pid());
    }

    related_pids
}

fn sum_memory_for_pids(
    system: &System,
    pids: impl IntoIterator<Item = Pid>,
) -> u64 {
    pids
        .into_iter()
        .filter_map(|pid| system.process(pid).map(|proc| proc.memory()))
        .sum()
}

fn is_descendant(
    process: &sysinfo::Process,
    root_pid: Pid,
    processes: &std::collections::HashMap<Pid, sysinfo::Process>,
) -> bool {
    let mut current = process.parent();
    while let Some(pid) = current {
        if pid == root_pid {
            return true;
        }
        current = processes.get(&pid).and_then(|proc| proc.parent());
    }
    false
}
