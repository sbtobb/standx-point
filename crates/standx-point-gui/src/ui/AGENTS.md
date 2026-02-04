## Architecture
- **Position**: UI views and components for the GUI layer.
- **Logic**: RootView composes status bar + sidebar + detail panels; modal forms update AppState.
- **Constraints**: Use GPUI and gpui-component; keep business logic in adapters/core.

## Members
- `account_form.rs`: Modal form to create an account from a private key and chain.
- `account_panel.rs`: Read-only account detail panel.
- `mod.rs`: UI module declarations and exports.
- `root.rs`: Root layout and view wiring.
- `sidebar.rs`: Task list sidebar component.
- `status_bar.rs`: Global status bar component.
- `task_card.rs`: Task list item card.
- `task_detail.rs`: Task detail panel.
- `task_form.rs`: Task creation/edit modal form.
