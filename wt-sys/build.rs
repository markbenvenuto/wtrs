use wt_build::{config_template_path, filelist_path, generate_config, generate_wiredtiger_h,
               parse_filelist, wiredtiger_h_template_path, wiredtiger_root};

use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let wt_root = wiredtiger_root();
    let filelist = filelist_path();
    let config_template = config_template_path();
    let wt_h_template = wiredtiger_h_template_path();

    // Generate headers for the C compiler's include path
    let config_output = out_dir.join("wiredtiger_config.h");
    generate_config(&config_template, &config_output)
        .expect("Failed to generate wiredtiger_config.h");

    let wiredtiger_h_output = out_dir.join("wiredtiger.h");
    generate_wiredtiger_h(&wt_h_template, &wiredtiger_h_output)
        .expect("Failed to generate wiredtiger.h");

    // Parse the filelist and compile the C library
    let files = parse_filelist(&filelist, &wt_root).expect("Failed to parse WiredTiger filelist");

    if files.is_empty() {
        panic!("No source files found in filelist");
    }

    let mut build = cc::Build::new();

    // TODO - use build.flag_if_supported
    build.flags(["-Wno-unused-function"]);

    build.include(&out_dir); // For generated wiredtiger_config.h
    build.include(wt_root.join("src/include"));

    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    if os == "macos" || os == "ios" {
        build.include(wt_root.join("oss/apple"));
    }

    // Linux requires _GNU_SOURCE for GNU extension functions (fallocate, sync_file_range, etc.)
    if os == "linux" || os == "android" {
        build.define("_GNU_SOURCE", None);
    }

    if os == "linux" && arch == "aarch64" {
        // Build with CRC32 support enabled
        build.flags(["-march=armv8-a+crc"]);
    }

    for file in &files {
        if file.exists() {
            build.file(file);
        } else {
            eprintln!("Warning: Source file not found: {}", file.display());
        }
    }

    build.compile("wt");

    println!("cargo:rerun-if-changed={}", filelist.display());
    println!("cargo:rerun-if-changed={}", config_template.display());
    println!("cargo:rerun-if-changed={}", wt_h_template.display());
}
