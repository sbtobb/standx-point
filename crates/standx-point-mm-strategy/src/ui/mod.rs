use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::Frame;

use crate::app::state::AppState;
use crate::state::storage::Storage;
use standx_point_mm_strategy::MarketDataHub;

pub mod components;

/// Main render function - called every frame
pub fn render(frame: &mut Frame, state: &AppState, storage: &Storage, market_data: &MarketDataHub) {
    let area = frame.area();

    if is_terminal_too_small(area) {
        render_overlay(frame, area);
        return;
    }

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
    components::status_bar::render(frame, status_area, state, market_data);
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

fn is_terminal_too_small(area: Rect) -> bool {
    area.width < 80 || area.height < 24
}

fn render_overlay(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);

    let [_, center, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Fill(1),
    ])
    .areas(area);

    let paragraph = Paragraph::new("Terminal too small (need 80x24)")
        .alignment(Alignment::Center)
        .block(Block::bordered());

    frame.render_widget(paragraph, center);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_is_terminal_too_small() {
        assert!(is_terminal_too_small(Rect::new(0, 0, 79, 24)));
        assert!(is_terminal_too_small(Rect::new(0, 0, 80, 23)));
        assert!(!is_terminal_too_small(Rect::new(0, 0, 80, 24)));
    }

    #[test]
    fn test_render_overlay_content() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                render_overlay(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let debug_str = format!("{:?}", buffer);
        assert!(debug_str.contains("Terminal too small (need 80x24)"));
    }
}
