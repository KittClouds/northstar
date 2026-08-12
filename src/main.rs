use gpui::{px, size, App, AppContext as _, Application, Bounds, WindowBounds, WindowOptions};
use gpui_component::Root;
use northstar_index_fund::{NorthstarApp, PrototypeRuntime};
use std::sync::Arc;

fn main() {
    let runtime = Arc::new(PrototypeRuntime::fixture());
    Application::new().run(move |cx: &mut App| {
        gpui_component::init(cx);
        cx.on_window_closed(|cx| cx.quit()).detach();
        let bounds = Bounds::centered(None, size(px(1_600.0), px(900.0)), cx);
        let app_runtime = Arc::clone(&runtime);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                window.set_window_title("Northstar Index Fund Prototype");
                let view = cx.new(|cx| NorthstarApp::new(Arc::clone(&app_runtime), cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("open Northstar prototype window");
    });
}
