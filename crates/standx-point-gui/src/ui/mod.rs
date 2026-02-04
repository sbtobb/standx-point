/// **Input**: UI submodules under `ui/`.
/// **Output**: Module declarations and public exports.
/// **Position**: UI module index.
/// **Update**: When UI modules are added, removed, or re-exported.
pub mod account_form;
// pub mod account_panel;
pub mod account_panel;
pub mod root;
pub mod sidebar;
pub mod status_bar;
pub mod task_card;
pub mod task_detail;
pub mod task_form;

pub use account_form::AccountForm;
// pub use account_panel::AccountPanel;
pub use root::RootView;
pub use sidebar::SidebarView;
pub use status_bar::StatusBar;
pub use task_card::TaskCard;
pub use task_detail::TaskDetailPanel;
pub use task_form::TaskForm;
