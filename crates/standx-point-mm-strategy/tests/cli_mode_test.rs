use std::process::Command;

#[test]
fn cli_mode_with_config_and_dry_run_works() {
    // Get the path to the binary from Cargo
    let binary_path = env!("CARGO_BIN_EXE_standx-point-mm-strategy");

    // Get the path to the test config file
    let config_path = format!(
        "{}/examples/single_task.yaml",
        env!("CARGO_MANIFEST_DIR")
    );

    // Spawn the process with --config and --dry-run flags
    let output = Command::new(binary_path)
        .arg("--config")
        .arg(config_path)
        .arg("--dry-run")
        .env("RUST_LOG", "error") // Reduce log output for test
        .output()
        .expect("Failed to start standx-point-mm-strategy binary");

    // Check that the process exited successfully
    assert!(
        output.status.success(),
        "Process exited with non-zero status: {}\nStdout: {}\nStderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Optional: Check for any error messages in stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        eprintln!("Warning: Process produced stderr output: {}", stderr);
    }
}
