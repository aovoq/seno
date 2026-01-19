use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalSize, Position, Rect, Size};

const INPUT_BAR_MIN: f64 = 76.0;
const INPUT_BAR_MAX: f64 = 520.0;

static INPUT_BAR_HEIGHT: AtomicU32 = AtomicU32::new(INPUT_BAR_MIN as u32);

#[derive(Clone, Copy)]
struct LayoutMetrics {
    width: f64,
    input_bar_height: f64,
    available_height: f64,
    panel_width: f64,
    last_panel_width: f64,
}

pub fn set_input_bar_height(height: f64) {
    let clamped = height.max(INPUT_BAR_MIN).min(INPUT_BAR_MAX).round() as u32;
    INPUT_BAR_HEIGHT.store(clamped, Ordering::SeqCst);
}

pub fn input_bar_height() -> f64 {
    INPUT_BAR_HEIGHT.load(Ordering::SeqCst) as f64
}

fn calculate_metrics(
    physical_size: PhysicalSize<u32>,
    scale_factor: f64,
    panel_count: usize,
) -> LayoutMetrics {
    let width = (physical_size.width as f64 / scale_factor).max(0.0).floor();
    let height = (physical_size.height as f64 / scale_factor).max(0.0).floor();
    let input_bar_height = input_bar_height();
    let available_height = (height - input_bar_height).max(100.0).floor();
    let count = panel_count.max(1) as f64;
    let panel_width = (width / count).floor();
    let last_panel_width = (width - panel_width * (count - 1.0)).max(0.0);

    LayoutMetrics {
        width,
        input_bar_height,
        available_height,
        panel_width,
        last_panel_width,
    }
}

pub fn apply_layout(
    app: &AppHandle,
    labels: &[&str],
    physical_size: PhysicalSize<u32>,
    scale_factor: f64,
) -> tauri::Result<()> {
    if labels.is_empty() {
        return Ok(());
    }

    let metrics = calculate_metrics(physical_size, scale_factor, labels.len());

    for (index, label) in labels.iter().enumerate() {
        if let Some(webview) = app.get_webview(label) {
            let width = if index + 1 == labels.len() {
                metrics.last_panel_width
            } else {
                metrics.panel_width
            };
            let x = (metrics.panel_width * index as f64).floor();
            let y = 0.0;
            let size = Size::Logical(LogicalSize {
                width,
                height: metrics.available_height,
            });
            let position = Position::Logical(LogicalPosition { x, y });
            let bounds = Rect { size, position };
            webview.set_bounds(bounds)?;
        }
    }

    if let Some(main_webview) = app.get_webview("main") {
        let size = Size::Logical(LogicalSize {
            width: metrics.width.floor(),
            height: metrics.input_bar_height.floor(),
        });
        let position = Position::Logical(LogicalPosition {
            x: 0.0,
            y: metrics.available_height,
        });
        let bounds = Rect { size, position };
        main_webview.set_bounds(bounds)?;
    }

    Ok(())
}
