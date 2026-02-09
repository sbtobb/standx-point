/*
[INPUT]:  TUI app state and rendering snapshots for UI components
[OUTPUT]: UI component render functions and module exports
[POS]:    TUI UI module root
[UPDATE]: 2026-02-09 Add UI module tree for refactor
[UPDATE]: 2026-02-09 Re-export panel draw functions
[UPDATE]: 2026-02-10 Re-export shared draw_tabs helper
*/

mod account;
mod layout;
mod logs;
mod orders;
mod positions;
mod task_list;

pub mod modal;

pub(in crate::tui) use account::draw_account_summary;
pub(in crate::tui) use layout::draw_tabs;
pub(in crate::tui) use logs::draw_logs;
pub(in crate::tui) use orders::draw_open_orders_table;
pub(in crate::tui) use positions::draw_positions_table;
pub(in crate::tui) use task_list::draw_task_list;
