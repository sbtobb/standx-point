/*
[INPUT]:  Help hint text and target render area.
[OUTPUT]: HelpText paragraph rendered into the given frame region.
[POS]:    TUI component for inline help/hint display.
[UPDATE]: Add reusable help text renderer with wrapping for long hints.
*/
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Render the help text area.
#[allow(dead_code)]
pub fn render(frame: &mut Frame, area: Rect, help_text: &str) {
    let block = Block::default()
        .title("Help")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::Cyan))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
