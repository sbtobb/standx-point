use crate::state::{Task, TaskStatus};
use gpui::*;

#[derive(IntoElement)]
pub struct TaskCard {
    task: Task,
    selected: bool,
}

impl TaskCard {
    pub fn new(task: Task, selected: bool) -> Self {
        Self { task, selected }
    }

    fn status_text(&self) -> &'static str {
        match self.task.status {
            TaskStatus::Draft => "Draft",
            TaskStatus::Pending => "Pending",
            TaskStatus::Running => "Running",
            TaskStatus::Paused => "Paused",
            TaskStatus::Stopped => "Stopped",
            TaskStatus::Failed => "Failed",
        }
    }
}

impl RenderOnce for TaskCard {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let status_color = match self.task.status {
            TaskStatus::Draft => rgb(0x808080),
            TaskStatus::Pending => rgb(0xFFD700),
            TaskStatus::Running => rgb(0x00FF00),
            TaskStatus::Paused => rgb(0xFFA500),
            TaskStatus::Stopped => rgb(0xFF0000),
            TaskStatus::Failed => rgb(0x8B0000),
        };

        div()
            .flex()
            .flex_col()
            .w_full()
            .p_2()
            .border_1()
            .border_color(if self.selected {
                rgb(0x4a90e2)
            } else {
                rgb(0x333333)
            })
            .bg(rgb(0x2a2a2a))
            .rounded_md()
            .hover(|s| s.bg(rgb(0x3a3a3a)))
            .cursor_pointer()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .child(self.task.name.clone()),
                    )
                    .child(div().w_2().h_2().rounded_full().bg(status_color)),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .mt_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0xaaaaaa))
                            .child(self.task.symbol.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(status_color)
                            .child(self.status_text()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .mt_2()
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .bg(rgb(0x444444))
                            .rounded_sm()
                            .text_xs()
                            .hover(|s| s.bg(rgb(0x555555)))
                            .child("Start")
                            .on_mouse_down(MouseButton::Left, |_, _, _cx| {
                                println!("Start clicked");
                            }),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .bg(rgb(0x444444))
                            .rounded_sm()
                            .text_xs()
                            .hover(|s| s.bg(rgb(0x555555)))
                            .child("Pause")
                            .on_mouse_down(MouseButton::Left, |_, _, _cx| {
                                println!("Pause clicked");
                            }),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .bg(rgb(0x444444))
                            .rounded_sm()
                            .text_xs()
                            .hover(|s| s.bg(rgb(0x555555)))
                            .child("Stop")
                            .on_mouse_down(MouseButton::Left, |_, _, _cx| {
                                println!("Stop clicked");
                            }),
                    ),
            )
    }
}
