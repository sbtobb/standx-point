use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::state::AppState;
use standx_point_mm_strategy::MarketDataHub;

/// Render the status bar at the top of the screen
pub fn render(frame: &mut Frame, area: Rect, state: &AppState, market_data: &MarketDataHub) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Blue));

    // Create the content lines
    let mut lines = vec![];

    // Title and mode line
    let mode_str = match state.mode {
        crate::app::state::AppMode::Normal => "NORMAL",
        crate::app::state::AppMode::Insert => "INSERT",
        crate::app::state::AppMode::Dialog => "DIALOG",
    };

    let mut title_spans = vec![
        Span::styled(
            "StandX MM Strategy",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("Mode: {}", mode_str),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("Sidebar: {:?}", state.sidebar_mode),
            Style::default().fg(Color::Green),
        ),
    ];

    // Add spinner if active
    if state.spinner_ticks > 0 {
        let spinner_frames = ['|', '/', '-', '\\'];
        let current_frame = spinner_frames[state.spinner_frame as usize];
        title_spans.push(Span::raw(" | "));
        title_spans.push(Span::styled(
            format!("Busy: {}", current_frame),
            Style::default().fg(Color::Red),
        ));
    }

    // Add price information
    title_spans.push(Span::raw(" | "));
    title_spans.push(Span::styled(
        "Prices: ",
        Style::default().fg(Color::Magenta),
    ));

    // Common symbols to display
    let symbols = ["BTC-USD", "ETH-USD"];
    for (i, symbol) in symbols.iter().enumerate() {
        if i > 0 {
            title_spans.push(Span::raw(", "));
        }

        if let Some(price) = market_data.get_price(symbol) {
            // Display mark price if available (or fallback to index price)
            let display_price = if price.mark_price != rust_decimal::Decimal::ZERO {
                price.mark_price
            } else {
                price.index_price
            };

            if display_price != rust_decimal::Decimal::ZERO {
                title_spans.push(Span::styled(
                    format!("{}: {:.2}", symbol, display_price),
                    Style::default(),
                ));
            } else {
                title_spans.push(Span::styled(
                    format!("{}: --", symbol),
                    Style::default().add_modifier(Modifier::DIM),
                ));
            }
        } else {
            title_spans.push(Span::styled(
                format!("{}: --", symbol),
                Style::default().add_modifier(Modifier::DIM),
            ));
        }
    }

    lines.push(Line::from(title_spans));

    // Status message or keypress flash
    if let Some((ref msg, _)) = state.keypress_flash {
        let flash_spans = vec![
            Span::styled("Key: ", Style::default().fg(Color::Yellow)),
            Span::styled(msg.clone(), Style::default().fg(Color::Yellow)),
        ];
        lines.push(Line::from(flash_spans));
    } else if let Some(ref msg) = state.status_message {
        let status_spans = vec![
            Span::styled("Status: ", Style::default().fg(Color::Blue)),
            Span::styled(msg.clone(), Style::default().fg(Color::Yellow)),
        ];
        lines.push(Line::from(status_spans));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}
