const SHADER_DIR: &str = "shaders";
const COMPUTE_SHADER_DIR: &str = "compute_shaders";

// Ensure that we recompile when shaders are changed
fn main() {
    println!("cargo:rerun-if-changed={}", SHADER_DIR);
    println!("cargo:rerun-if-changed={}", COMPUTE_SHADER_DIR);
}
