use gpui::{px, size, App, AppContext as _, Application, Bounds, WindowBounds, WindowOptions};
use gpui_component::Root;
use northstar_index_fund::operating::{NorthstarRuntime, OperatingMode};
use northstar_index_fund::OperatingApp;
use std::sync::Arc;

fn main() {
    let ledger_root = std::env::var_os("NORTHSTAR_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join("Northstar")
                .join("ledger-v1")
        });
    let window_title =
        std::env::var("NORTHSTAR_WINDOW_TITLE").unwrap_or_else(|_| "Northstar Index Office".into());
    let runtime = NorthstarRuntime::with_ledger(OperatingMode::LiveData, &ledger_root)
        .unwrap_or_else(|error| {
            eprintln!("NORTHSTAR_LEDGER_UNAVAILABLE root={ledger_root:?} error={error}");
            NorthstarRuntime::empty(OperatingMode::LiveData).0
        });
    Application::new().run(move |cx: &mut App| {
        gpui_component::init(cx);
        cx.on_window_closed(|cx| cx.quit()).detach();
        let bounds = Bounds::centered(None, size(px(1_440.0), px(780.0)), cx);
        let app_runtime = Arc::clone(&runtime);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                window.set_window_title(&window_title);
                let view = cx.new(|cx| OperatingApp::new(Arc::clone(&app_runtime), window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("open Northstar operating window");
    });
}
