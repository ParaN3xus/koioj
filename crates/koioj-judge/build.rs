use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap())
        .ancestors()
        .nth(3) //  out/ -> <hash>/ -> build/ -> profile/
        .unwrap()
        .to_path_buf();

    let src_path = "src/judger.cpp";
    let bin_name = "judger";
    let bin_path = Path::new(&out_dir).join(bin_name);

    let status = Command::new("g++")
        .args(&[
            src_path,
            "-o",
            bin_path.to_str().unwrap(),
            "-std=c++17",
            "-O2",
            "-Wall",
            "-static-libstdc++",
        ])
        .status()
        .expect("Failed to execute g++");

    if !status.success() {
        panic!("Compilation of judger.cpp failed");
    }

    println!("cargo:rerun-if-changed={}", src_path);
}
