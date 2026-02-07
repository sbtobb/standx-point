use ratatui::Frame;
/// **Input**: ModalType variants and ratatui layout/style primitives.
/// **Output**: Modal dialog rendering via ratatui widgets.
/// **Position**: Modal rendering entrypoint in the TUI component layer.
/// **Update**: Add account form modal rendering.
/// **Update**: Handle confirm modal action context.
/// **Update**: Add task form modal rendering.
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::state::ModalType;
use crate::ui::components::account_form;
use crate::ui::components::task_form;

/// Render a modal dialog
pub fn render(frame: &mut Frame, area: Rect, modal: &ModalType) {
    // Clear the background
    frame.render_widget(Clear, area);

    match modal {
        ModalType::Help => {
            // Help modal is handled by the help component
            // This should not be reached if help is shown via state.show_help
        }
        ModalType::Confirm {
            title,
            message,
            action: _,
        } => {
            render_confirmation(frame, area, title, message);
        }
        ModalType::AccountForm { form, is_edit } => {
            account_form::render(frame, area, form, *is_edit);
        }
        ModalType::TaskForm { form, is_edit } => {
            task_form::render(frame, area, form, *is_edit);
        }
    }
}

/// Render a confirmation dialog
fn render_confirmation(frame: &mut Frame, area: Rect, title: &str, message: &str) {
    // Create a centered popup
    let popup_area = centered_rect(60, 30, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} ", title))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("⚠ ", Style::default().fg(Color::Yellow)),
            Span::styled(message, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "Are you sure you want to proceed?",
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "    [y] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Yes  ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "[n] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled("No  ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "[Esc] ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Cancel", Style::default().fg(Color::Cyan)),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, popup_area);
}

/// Create a centered rect for modals
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
