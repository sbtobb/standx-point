use std::process::Command;

#[test]
fn tui_mode_starts_and_exits_cleanly() {
    // Get the path to the binary from Cargo
    let binary_path = env!("CARGO_BIN_EXE_standx-point-mm-strategy");

    // Spawn the process with test environment variables
    let output = Command::new(binary_path)
        .env("STANDX_TUI_TEST_EXIT_AFTER_TICKS", "1")
        .env("RUST_LOG", "error") // Reduce log output for test
        .output()
        .expect("Failed to start standx-point-mm-strategy binary");

    // Check that the process exited successfully
    assert!(
        output.status.success(),
        "Process exited with non-zero status: {}",
        output.status
    );

    // Optional: Check for any error messages in stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        eprintln!("Warning: Process produced stderr output: {}", stderr);
    }
}
