extern crate bindgen;
extern crate cmake;

use cmake::Config;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let lgbm_root = Path::new(&out_dir).join("lightgbm");

    // copy source code
    if !lgbm_root.exists() {
        let status = if target.contains("windows") {
            Command::new("cmd")
                .args(&[
                    "/C",
                    "echo D | xcopy /S /Y lightgbm",
                    lgbm_root.to_str().unwrap(),
                ])
                .status()
        } else {
            Command::new("cp")
                .args(&["-r", "lightgbm", lgbm_root.to_str().unwrap()])
                .status()
        };
        if let Some(err) = status.err() {
            panic!(
                "Failed to copy ./lightgbm to {}: {}",
                lgbm_root.display(),
                err
            );
        }
    }

    let mut dst = Config::new(&lgbm_root);

    // CMake
    #[cfg(feature = "cuda")]
    let dst = dst
        .profile("Release")
        .define("CMAKE_CXX_STANDARD", "14")
        .define("BUILD_STATIC_LIB", "ON")
        .define("USE_CUDA", "1")
        .define("USE_CUDA_EXP", "1");

    #[cfg(not(feature = "cuda"))]
    let dst = dst
        .profile("Release")
        .define("CMAKE_CXX_STANDARD", "14")
        .define("BUILD_STATIC_LIB", "ON");

    #[cfg(target_os = "macos")]
    let dst = dst
        .define("CMAKE_C_COMPILER", "/opt/homebrew/opt/llvm/bin/clang")
        .define("CMAKE_CXX_COMPILER", "/opt/homebrew/opt/llvm/bin/clang++")
        .define("OPENMP_LIBRARIES", "/opt/homebrew/opt/llvm/lib")
        .define("OPENMP_INCLUDES", "/opt/homebrew/opt/llvm/include");

    let dst = dst.build();

    // bindgen build
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_args(&["-x", "c++", "-std=c++14"])
        .clang_arg(format!("-I{}", lgbm_root.join("include").display()));

    #[cfg(target_os = "linux")]
    let bindings = bindings
        .clang_arg(format!("-I/usr/include/c++/11"))
        .clang_arg(format!("-I/usr/include/x86_64-linux-gnu/c++/11"));

    #[cfg(feature = "cuda")]
    let bindings = bindings.clang_arg("-I/usr/local/cuda/include");

    let bindings = bindings.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings.");

    // link to appropriate C++ lib
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=c++");
        println!("cargo:rustc-link-lib=dylib=omp");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-lib=stdc++");
        println!("cargo:rustc-link-lib=dylib=gomp");
    }

    println!("cargo:rustc-link-search={}", out_path.join("lib").display());
    println!("cargo:rustc-link-search=native={}", dst.display());
    if target.contains("windows") {
        println!("cargo:rustc-link-lib=static=lib_lightgbm");
    } else {
        println!("cargo:rustc-link-lib=static=_lightgbm");
    }

    #[cfg(feature = "cuda")]
    {
        println!("cargo:rustc-link-search={}", "/usr/local/cuda/lib64");
        println!("cargo:rustc-link-lib=static=cudart_static");
    }
}
