use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    let input_path = PathBuf::from("assets/shaders/");
    let paths = fs::read_dir(&input_path).unwrap();
    println!("cargo:rerun-if-changed={}", input_path.to_string_lossy());

    compile_shaders(paths, PathBuf::new(), out_dir);
}

fn compile_shaders(paths: fs::ReadDir, parent_path: PathBuf, out_dir: String) {
    for entry in paths {
        let entry = match entry {
            Ok(path) => path,
            Err(_) => continue,
        };
        let shader_file_name = entry.file_name();
        let shader_path = entry.path();
        if shader_path.is_dir() {
            let mut child_path = parent_path.clone();
            child_path.push(shader_file_name);
            compile_shaders(
                fs::read_dir(shader_path).unwrap(),
                child_path,
                out_dir.clone(),
            );
            continue;
        }
        if !shader_path.is_file() {
            continue;
        }

        let mut input_path = PathBuf::new();
        input_path.push("assets");
        input_path.push("shaders");
        input_path.push(&parent_path);
        input_path.push(&shader_file_name);

        let mut output_file_name = shader_file_name.clone();
        output_file_name.push(".spv");
        let mut output_path = PathBuf::new();
        output_path.push(&out_dir);
        output_path.push(&parent_path);
        output_path.push(&output_file_name);

        let shader_file_name = shader_file_name.to_string_lossy();
        // glslc can't automatically create directories, so we're just going to pick a flat structure
        let shader_compile_result = Command::new("glslc")
            .arg(&input_path)
            .arg("-o")
            .arg(&output_path)
            .status()
            .unwrap();

        if !shader_compile_result.success() {
            panic!(
                "Shader compilation for {} failed: {}",
                shader_file_name, shader_compile_result
            );
        }
    }
}
