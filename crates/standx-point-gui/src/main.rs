use gpui::*;
use gpui_component::Root;
use standx_point_gui::ui::RootView;

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1200.0), px(800.0)),
                cx,
            ))),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let view = cx.new(|cx| RootView::build(cx));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .unwrap();
    });
}
