use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=public");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=index.html");
    println!("cargo:rerun-if-changed=vite.config.ts");

    println!("cargo:rerun-if-changed=../../package.json");
    println!("cargo:rerun-if-changed=../../postcss.config.js");
    println!("cargo:rerun-if-changed=../../tailwind.config.js");
    println!("cargo:rerun-if-changed=../../tsconfig.app.json");
    println!("cargo:rerun-if-changed=../../tsconfig.json");
    println!("cargo:rerun-if-changed=../../tsconfig.node.json");
    println!("cargo:rerun-if-changed=../../yarn.lock");

    let yarn_command = if cfg!(target_os = "windows") {
        "yarn.cmd"
    } else {
        "yarn"
    };

    let status = Command::new(yarn_command)
        .args(&["build"])
        .status()
        .expect("Failed to execute yarn build");

    if !status.success() {
        panic!("yarn build failed");
    }

    let dist_dir = Path::new("dist");
    if !dist_dir.exists() {
        println!("cargo:warning=dist directory not found, creating empty DIST_FILES");
        generate_empty_map();
        return;
    }

    let mut entries = Vec::new();
    scan_dir(dist_dir, "", &mut entries);
    generate_map(&entries);
}

fn generate_map(entries: &[(String, String)]) {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("static_files.rs");

    let mut code = String::from("use phf::phf_map;\n\n");
    code.push_str("pub static DIST_FILES: phf::Map<&'static str, &'static [u8]> = phf_map! {\n");

    for (web_path, file_path) in entries {
        code.push_str(&format!(
            "    \"{}\" => include_bytes!(r\"{}\"),\n",
            web_path, file_path
        ));
    }

    code.push_str("};\n");

    fs::write(dest_path, code).unwrap();
}

fn generate_empty_map() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("static_files.rs");

    let code = r#"use phf::phf_map;

pub static DIST_FILES: phf::Map<&'static str, &'static [u8]> = phf_map! {};
"#;

    fs::write(dest_path, code).unwrap();
}

fn scan_dir(dir: &Path, prefix: &str, entries: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = entry.file_name().into_string().unwrap();

        if path.is_dir() {
            let new_prefix = format!("{}/{}", prefix, name);
            scan_dir(&path, &new_prefix, entries);
        } else {
            let web_path = format!("{}/{}", prefix, name);
            let file_path = path.canonicalize().unwrap().display().to_string();
            entries.push((web_path, file_path));
        }
    }
}
