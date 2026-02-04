## Architecture
- **Position**: GUI crate for StandX Point, providing a visual interface for monitoring and managing points/strategies.
- **Logic**: GPUI App -> Views -> State Management -> `standx-point-adapter`.
- **Constraints**: Use GPUI for all UI; follow the established design system in `gpui-component`; business logic should mostly reside in adapters or other core crates.

## Members
- `Cargo.toml`: Crate dependencies and build configuration.
- `src/main.rs`: Entry point for the GPUI application.
- `src/lib.rs`: Library entry point (currently empty).

## Conventions (Optional)
- Use `anyhow::Result` for error handling.
- Prefer `RenderOnce` for simple components and `Render` for stateful views.
- Async operations should be handled via `cx.spawn`.
