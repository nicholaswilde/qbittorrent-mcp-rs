use std::process::{Command, Stdio};
use tempfile::tempdir;

#[test]
fn test_main_coverage_triggers() {
    let bin_path = env!("CARGO_BIN_EXE_qbittorrent-mcp-rs");
    let dir = tempdir().unwrap();

    // Test with all flags and file logging
    let mut child = Command::new(bin_path)
        .arg("--server-mode")
        .arg("stdio")
        .arg("--qbittorrent-host")
        .arg("http://localhost")
        .arg("--qbittorrent-port")
        .arg("8080")
        .arg("--qbittorrent-username")
        .arg("admin")
        .arg("--qbittorrent-password")
        .arg("password")
        .arg("--log-file-enable")
        .arg("--log-dir")
        .arg(dir.path().to_str().unwrap())
        .arg("--log-filename")
        .arg("test")
        .arg("--log-rotate")
        .arg("hourly")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn process");

    // Give it a moment to initialize
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Just kill it, we just want to trigger the init code
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_main_different_rotations() {
    let bin_path = env!("CARGO_BIN_EXE_qbittorrent-mcp-rs");
    let dir = tempdir().unwrap();

    let rotations = vec!["never", "daily"];
    for rotation in rotations {
        let mut child = Command::new(bin_path)
            .arg("--log-file-enable")
            .arg("--log-dir")
            .arg(dir.path().to_str().unwrap())
            .arg("--log-rotate")
            .arg(rotation)
            .stdin(Stdio::piped())
            .spawn()
            .expect("Failed to spawn process");

        std::thread::sleep(std::time::Duration::from_millis(100));
        let _ = child.kill();
        let _ = child.wait();
    }
}
