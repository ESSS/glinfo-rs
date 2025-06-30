use std::process::Command;

#[test]
fn test_gl_info() {
    let output = Command::new(env!("CARGO_BIN_EXE_glinfo")).output().unwrap();
    println!("{}",env!("CARGO_BIN_EXE_glinfo"));
    println!("OUTPUT: {}", String::from_utf8(output.stdout).unwrap());
}
