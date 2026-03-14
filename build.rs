use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=ffi/enry/main.go");
    println!("cargo:rerun-if-changed=ffi/enry/go.mod");
    println!("cargo:rerun-if-changed=ffi/enry/go.sum");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let archive_path = out_dir.join("libenry_classifier.a");

    let status = Command::new("go")
        .args([
            "build",
            "-buildmode=c-archive",
            "-trimpath",
            "-ldflags=-s -w",
            "-o",
        ])
        .arg(&archive_path)
        .arg(".")
        .current_dir("ffi/enry")
        .status()
        .expect("failed to run `go build` for enry classifier ffi");

    if !status.success() {
        panic!("`go build` for enry classifier ffi failed");
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=enry_classifier");

    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=dl");
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=pthread");
    }
}
