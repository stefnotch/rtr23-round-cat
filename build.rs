use std::process::Command;
use std::{env, fs};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    let paths = fs::read_dir("assets/shaders/").unwrap();

    for shader_path in paths {
        let shader_file_name: std::ffi::OsString = shader_path.unwrap().file_name();
        let shader = shader_file_name.to_string_lossy();
        // glslc can't automatically create directories, so we're just going to pick a flat structure
        let shader_compile_result = Command::new("glslc")
            .arg(&format!("assets/shaders/{}", shader))
            .arg("-o")
            .arg(&format!("{}/{}.spv", out_dir, shader))
            .status()
            .unwrap();

        if !shader_compile_result.success() {
            panic!(
                "Shader compilation for {} failed: {}",
                shader, shader_compile_result
            );
        }

        println!("cargo:rerun-if-changed=assets/shaders/{}", shader);
    }
}
