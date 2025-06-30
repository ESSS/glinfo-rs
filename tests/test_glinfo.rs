use std::process::Command;

#[test]
fn test_gl_info() {
    let output = Command::new(env!("CARGO_BIN_EXE_glinfo")).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Vendor:"));
}

#[test]
fn test_gl_info_file() {
    _ = Command::new(env!("CARGO_BIN_EXE_glinfo")).args(["-f", "out.txt"]).output().unwrap();
    let stdout = std::fs::read_to_string("out.txt").unwrap();
    assert!(stdout.contains("Vendor:"));
}
