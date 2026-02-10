/*
[INPUT]:  Crossterm stdout, terminal raw mode, ratatui backend
[OUTPUT]: TerminalGuard managing alternate screen lifecycle
[POS]:    TUI terminal lifecycle guard
[UPDATE]: 2026-02-09 Move TerminalGuard from tui/mod.rs
*/

use std::io;

use anyhow::Result;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{ExecutableCommand, terminal};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

pub(super) struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    pub(super) fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        stdout.execute(EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub(super) fn draw<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.terminal.show_cursor();
        let mut stdout = io::stdout();
        let _ = stdout.execute(LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}
