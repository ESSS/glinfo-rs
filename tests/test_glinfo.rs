use std::process::Command;

#[test]
fn test_gl_info() {
    let output = Command::new("target/debug/glinfo").output().unwrap();
    println!("{}", String::from_utf8(output.stdout).unwrap());
}
