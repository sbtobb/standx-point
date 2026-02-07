/// **Input**: Option list plus keyboard events for navigation/confirmation.
/// **Output**: Selected value on confirm and list rendering via ratatui.
/// **Position**: Reusable TUI component for single-choice selection.
/// **Update**: Revisit when selection behavior or styling changes.
use std::fmt::Display;

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

#[derive(Debug, Clone)]
pub struct SingleSelect<T> {
    title: String,
    options: Vec<T>,
    cursor: usize,
    selected: Option<usize>,
}

impl<T> SingleSelect<T>
where
    T: Clone + Display,
{
    pub fn new(title: impl Into<String>, options: Vec<T>) -> Self {
        Self {
            title: title.into(),
            options,
            cursor: 0,
            selected: None,
        }
    }

    pub fn options(&self) -> &[T] {
        &self.options
    }

    pub fn set_options(&mut self, options: Vec<T>) {
        self.options = options;
        if self.options.is_empty() {
            self.cursor = 0;
            self.selected = None;
        } else {
            if self.cursor >= self.options.len() {
                self.cursor = self.options.len() - 1;
            }
            if let Some(selected) = self.selected {
                if selected >= self.options.len() {
                    self.selected = None;
                }
            }
        }
    }

    pub fn set_selected_index(&mut self, index: usize) {
        if !self.options.is_empty() && index < self.options.len() {
            self.cursor = index;
            self.selected = Some(index);
        }
    }

    pub fn set_selected_value(&mut self, value: &T)
    where
        T: PartialEq,
    {
        if let Some(index) = self.options.iter().position(|option| option == value) {
            self.set_selected_index(index);
        }
    }

    pub fn cursor_index(&self) -> usize {
        self.cursor
    }

    #[cfg(test)]
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected(&self) -> Option<T> {
        self.selected
            .and_then(|index| self.options.get(index))
            .cloned()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<T> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_up();
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_down();
                None
            }
            KeyCode::Enter => self.confirm(),
            _ => None,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let items = if self.options.is_empty() {
            vec![ListItem::new(Line::from("No options"))]
        } else {
            self.options
                .iter()
                .map(|option| ListItem::new(Line::from(option.to_string())))
                .collect()
        };

        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        let mut state = ListState::default();
        if !self.options.is_empty() {
            state.select(Some(self.cursor));
        }

        frame.render_stateful_widget(list, area, &mut state);
    }

    fn move_up(&mut self) {
        if self.options.is_empty() {
            return;
        }
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn move_down(&mut self) {
        if self.options.is_empty() {
            return;
        }
        if self.cursor + 1 < self.options.len() {
            self.cursor += 1;
        }
    }

    fn confirm(&mut self) -> Option<T> {
        if self.options.is_empty() {
            return None;
        }
        self.selected = Some(self.cursor);
        self.options.get(self.cursor).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    fn key_event(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn move_down_respects_bounds() {
        let mut select = SingleSelect::new("Symbols", vec!["BTC", "ETH", "SOL"]);

        select.handle_key(key_event(KeyCode::Down));
        assert_eq!(select.cursor_index(), 1);

        select.handle_key(key_event(KeyCode::Down));
        select.handle_key(key_event(KeyCode::Down));
        assert_eq!(select.cursor_index(), 2);

        select.handle_key(key_event(KeyCode::Down));
        assert_eq!(select.cursor_index(), 2);
    }

    #[test]
    fn move_up_respects_bounds() {
        let mut select = SingleSelect::new("Symbols", vec!["BTC", "ETH", "SOL"]);

        select.handle_key(key_event(KeyCode::Down));
        select.handle_key(key_event(KeyCode::Down));
        assert_eq!(select.cursor_index(), 2);

        select.handle_key(key_event(KeyCode::Up));
        select.handle_key(key_event(KeyCode::Up));
        select.handle_key(key_event(KeyCode::Up));
        assert_eq!(select.cursor_index(), 0);
    }

    #[test]
    fn confirm_returns_selected_value() {
        let mut select = SingleSelect::new("Risk", vec!["Low", "Medium", "High"]);

        select.handle_key(key_event(KeyCode::Down));
        let selected = select.handle_key(key_event(KeyCode::Enter));

        assert_eq!(selected, Some("Medium"));
        assert_eq!(select.selected_index(), Some(1));
    }
}
