use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::Frame;

use crate::app::state::AppState;
use crate::state::storage::Storage;

pub mod components;

/// Main render function - called every frame
pub fn render(frame: &mut Frame, state: &AppState, storage: &Storage) {
    let area = frame.area();

    // Split the screen into main sections
    let [status_area, main_area, menu_area] = Layout::vertical([
        Constraint::Length(3), // Status bar
        Constraint::Fill(1),   // Main content
        Constraint::Length(1), // Menu bar
    ])
    .areas(area);

    // Split main area into sidebar and detail
    let [sidebar_area, detail_area] = Layout::horizontal([
        Constraint::Percentage(30), // Sidebar
        Constraint::Fill(1),        // Detail view
    ])
    .areas(main_area);

    // Render each component
    components::status_bar::render(frame, status_area, state);
    components::sidebar::render(frame, sidebar_area, state, storage);
    components::detail_view::render(frame, detail_area, state, storage);
    components::menu_bar::render(frame, menu_area, state);

    // Render modal on top if present
    if let Some(ref modal) = state.modal {
        components::modal::render(frame, area, modal);
    }

    // Render help overlay if shown
    if state.show_help {
        components::help::render(frame, area);
    }
}
