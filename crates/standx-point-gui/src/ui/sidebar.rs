use super::task_card::TaskCard;
use crate::state::AppState;
use gpui::*;

pub struct SidebarView {
    state: Entity<AppState>,
    selected_task_id: Option<String>,
}

impl SidebarView {
    pub fn new(state: Entity<AppState>, _cx: &mut App) -> Self {
        Self {
            state,
            selected_task_id: None,
        }
    }
}

impl Render for SidebarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);

        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(rgb(0x252526))
            .child(
                div()
                    .p_4()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xcccccc))
                    .child("TASKS"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_2()
                    .children(state.tasks.iter().map(|task| {
                        let is_selected = self.selected_task_id.as_ref() == Some(&task.id);
                        let task_id = task.id.clone();

                        div()
                            .child(TaskCard::new(task.clone(), is_selected))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.selected_task_id = Some(task_id.clone());
                                    cx.notify();
                                }),
                            )
                    })),
            )
    }
}
