const SHADER_DIR: &str = "shaders";

// Ensure that we recompile when shaders are changed
fn main() {
    println!("cargo:rerun-if-changed={}", SHADER_DIR);
}
