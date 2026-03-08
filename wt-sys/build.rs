extern crate cc;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Get the path to the WiredTiger source directory (git submodule at wt-sys/src/wiredtiger)
fn wiredtiger_root() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/wiredtiger")
}

fn filelist_path() -> PathBuf {
    wiredtiger_root().join("dist/filelist")
}

fn config_template_path() -> PathBuf {
    wiredtiger_root().join("cmake/configs/wiredtiger_config.h.in")
}

fn wiredtiger_h_template_path() -> PathBuf {
    wiredtiger_root().join("src/include/wiredtiger.h.in")
}

// Version info
const VERSION_MAJOR: &str = "12";
const VERSION_MINOR: &str = "0";
const VERSION_PATCH: &str = "0";
const VERSION_STRING: &str = "\"WiredTiger 12.0.0 (Rust build)\"";

/// Determine the current CPU architecture filter
fn get_arch_filter() -> &'static str {
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    match arch.as_str() {
        "x86" | "x86_64" => "X86_HOST",
        "aarch64" => "ARM64_HOST",
        "powerpc" | "powerpc64" => "POWERPC_HOST",
        "riscv64" => "RISCV64_HOST",
        "loongarch64" => "LOONGARCH64_HOST",
        "s390x" => "ZSERIES_HOST",
        _ => "",
    }
}

/// Determine the current OS filter
fn get_os_filter() -> &'static str {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match os.as_str() {
        "macos" | "ios" => "DARWIN_HOST",
        "linux" | "android" => "LINUX_HOST",
        "windows" => "WINDOWS_HOST",
        _ => "",
    }
}

/// Check if we're building for a POSIX-compatible system
fn is_posix() -> bool {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    matches!(
        os.as_str(),
        "macos" | "ios" | "linux" | "android" | "freebsd" | "openbsd" | "netbsd" | "dragonfly"
    )
}

/// Check if a file with a given filter should be included in the build
fn should_include_file(filter: &str, arch_filter: &str, os_filter: &str, is_posix: bool) -> bool {
    if filter.is_empty() {
        // No filter means always include
        return true;
    }

    match filter {
        // Architecture-specific filters
        "X86_HOST" | "ARM64_HOST" | "POWERPC_HOST" | "RISCV64_HOST" | "LOONGARCH64_HOST"
        | "ZSERIES_HOST" => filter == arch_filter,
        // OS-specific filters
        "DARWIN_HOST" | "LINUX_HOST" | "WINDOWS_HOST" => filter == os_filter,
        // POSIX filter (includes both Darwin and Linux)
        "POSIX_HOST" => is_posix,
        // Unknown filter - exclude by default
        _ => {
            eprintln!("Warning: Unknown filter '{}', excluding file", filter);
            false
        }
    }
}

/// Parse the filelist and return a list of source files applicable to the current platform
fn parse_filelist(filelist_path: &Path, wt_root: &Path) -> Result<Vec<PathBuf>, String> {
    let content = fs::read_to_string(filelist_path).map_err(|e| {
        format!(
            "Failed to read filelist '{}': {}",
            filelist_path.display(),
            e
        )
    })?;

    let arch_filter = get_arch_filter();
    let os_filter = get_os_filter();
    let posix = is_posix();

    let mut files = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split the line into file path and optional filter
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let file_path = parts[0];
        let filter = parts.get(1).copied().unwrap_or("");

        // Only include .c files (skip .S assembly files for now as cc crate handles them differently)
        if !file_path.ends_with(".c") {
            continue;
        }

        // Check if this file should be included based on the filter
        if should_include_file(filter, arch_filter, os_filter, posix) {
            let full_path = wt_root.join(file_path);
            files.push(full_path);
        }
    }

    Ok(files)
}

/// Get the set of enabled features based on the target platform
fn get_enabled_features() -> HashMap<&'static str, bool> {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    let is_darwin = os == "macos" || os == "ios";
    let is_linux = os == "linux" || os == "android";
    let is_posix = is_darwin || is_linux;
    let is_x86 = arch == "x86" || arch == "x86_64";
    let is_arm64 = arch == "aarch64";

    let mut features = HashMap::new();

    // Debug/diagnostic features (disabled by default for release builds)
    features.insert("HAVE_ATTACH", false);
    features.insert("HAVE_CALL_LOG", false);
    features.insert("HAVE_DIAGNOSTIC", false);
    features.insert("HAVE_ERROR_LOG", false);
    features.insert("HAVE_REF_TRACK", false);
    features.insert("HAVE_UNITTEST", false);
    features.insert("CODE_COVERAGE_MEASUREMENT", false);
    features.insert("INLINE_FUNCTIONS_INSTEAD_OF_MACROS", false);
    features.insert("HAVE_UNITTEST_ASSERTS", false);

    // Builtin extensions (disabled by default - can be enabled via features later)
    features.insert("HAVE_BUILTIN_EXTENSION_LZ4", false);
    features.insert("HAVE_BUILTIN_EXTENSION_SNAPPY", false);
    features.insert("HAVE_BUILTIN_EXTENSION_ZLIB", false);
    features.insert("HAVE_BUILTIN_EXTENSION_ZSTD", false);
    features.insert("HAVE_BUILTIN_EXTENSION_IAA", false);
    features.insert("HAVE_BUILTIN_EXTENSION_SODIUM", false);
    features.insert("HAVE_BUILTIN_EXTENSION_KEY_PROVIDER", false);

    // POSIX functions
    features.insert("HAVE_FALLOCATE", is_linux);
    features.insert("HAVE_FDATASYNC", is_posix);
    features.insert("HAVE_CLOCK_GETTIME", is_posix);
    features.insert("HAVE_GETTIMEOFDAY", is_posix);
    features.insert("HAVE_POSIX_FADVISE", is_linux);
    features.insert("HAVE_POSIX_FALLOCATE", is_linux);
    features.insert("HAVE_POSIX_MADVISE", is_posix);
    features.insert("HAVE_POSIX_MEMALIGN", is_posix);
    features.insert("HAVE_PTHREAD_COND_MONOTONIC", is_linux);
    features.insert("HAVE_SETRLIMIT", is_posix);
    features.insert("HAVE_SYNC_FILE_RANGE", is_linux);
    features.insert("HAVE_TIMER_CREATE", is_linux);

    // Libraries
    features.insert("HAVE_LIBDL", is_posix);
    features.insert("HAVE_LIBCXX", true);
    features.insert("HAVE_LIBPTHREAD", is_posix);
    features.insert("HAVE_LIBRT", is_linux);
    features.insert("HAVE_LIBACCEL_CONFIG", false);
    features.insert("HAVE_LIBLZ4", false);
    features.insert("HAVE_LIBMEMKIND", false);
    features.insert("ENABLE_MEMKIND", false);
    features.insert("HAVE_LIBSNAPPY", false);
    features.insert("ENABLE_ANTITHESIS", false);
    features.insert("HAVE_LIBZ", false);
    features.insert("HAVE_LIBZSTD", false);
    features.insert("HAVE_LIBQPL", false);
    features.insert("HAVE_LIBSODIUM", false);

    // Hardware/architecture specific
    features.insert("HAVE_X86INTRIN_H", is_x86);
    features.insert("HAVE_ARM_NEON_INTRIN_H", is_arm64);
    features.insert("HAVE_RCPC", false);
    features.insert("HAVE_NO_CRC32_HARDWARE", false);
    features.insert("WORDS_BIGENDIAN", false);

    // Standalone build
    features.insert("WT_STANDALONE_BUILD", true);

    features
}

/// Get the spinlock type based on the target platform
fn get_spinlock_type() -> &'static str {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match os.as_str() {
        "macos" | "ios" | "linux" | "android" => "SPINLOCK_PTHREAD_MUTEX",
        "windows" => "SPINLOCK_MSVC",
        _ => "SPINLOCK_PTHREAD_MUTEX",
    }
}

/// Process a #cmakedefine line
fn process_cmakedefine(line: &str, features: &HashMap<&str, bool>) -> String {
    // Parse: #cmakedefine SYMBOL [value]
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return format!("/* {} */", line);
    }

    let symbol = parts[1];
    let value = parts.get(2).copied();

    let is_enabled = features.get(symbol).copied().unwrap_or(false);

    match value {
        Some("0") => {
            // #cmakedefine SYMBOL 0 -> always #define SYMBOL 0
            format!("#define {} 0", symbol)
        }
        Some("1") => {
            if is_enabled {
                format!("#define {} 1", symbol)
            } else {
                format!("/* #undef {} */", symbol)
            }
        }
        Some(v) if v.starts_with('@') => {
            // Variable substitution like @SPINLOCK_TYPE_CONFIG_VAR@
            if is_enabled {
                format!("#define {} {}", symbol, v)
            } else {
                format!("/* #undef {} */", symbol)
            }
        }
        None => {
            // #cmakedefine SYMBOL (no value)
            if is_enabled {
                format!("#define {}", symbol)
            } else {
                format!("/* #undef {} */", symbol)
            }
        }
        Some(v) => {
            // Other value
            if is_enabled {
                format!("#define {} {}", symbol, v)
            } else {
                format!("/* #undef {} */", symbol)
            }
        }
    }
}

/// Generate wiredtiger_config.h from the template
fn generate_config(template_path: &Path, output_path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(template_path).map_err(|e| {
        format!(
            "Failed to read config template '{}': {}",
            template_path.display(),
            e
        )
    })?;

    let features = get_enabled_features();
    let spinlock_type = get_spinlock_type();

    let mut output_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("#cmakedefine") {
            output_lines.push(process_cmakedefine(trimmed, &features));
        } else {
            // Handle @VAR@ substitutions
            let mut processed = line.to_string();
            processed = processed.replace("@VERSION_MAJOR@", VERSION_MAJOR);
            processed = processed.replace("@VERSION_MINOR@", VERSION_MINOR);
            processed = processed.replace("@VERSION_PATCH@", VERSION_PATCH);
            processed = processed.replace("@SPINLOCK_TYPE_CONFIG_VAR@", spinlock_type);
            output_lines.push(processed);
        }
    }

    let output_content = output_lines.join("\n");
    fs::write(output_path, output_content).map_err(|e| {
        format!(
            "Failed to write config file '{}': {}",
            output_path.display(),
            e
        )
    })?;

    Ok(())
}

/// Generate wiredtiger.h from the template
fn generate_wiredtiger_h(template_path: &Path, output_path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(template_path).map_err(|e| {
        format!(
            "Failed to read wiredtiger.h template '{}': {}",
            template_path.display(),
            e
        )
    })?;

    // Perform version substitutions
    let output_content = content
        .replace("@VERSION_MAJOR@", VERSION_MAJOR)
        .replace("@VERSION_MINOR@", VERSION_MINOR)
        .replace("@VERSION_PATCH@", VERSION_PATCH)
        .replace("@VERSION_STRING@", VERSION_STRING);

    fs::write(output_path, output_content).map_err(|e| {
        format!(
            "Failed to write wiredtiger.h '{}': {}",
            output_path.display(),
            e
        )
    })?;

    Ok(())
}

fn gen_bindings(wiredtiger_h_output: &str) {
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header(wiredtiger_h_output)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn main() {
    // Get paths
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let wt_root = wiredtiger_root();
    let filelist = filelist_path();
    let config_template = config_template_path();
    let wt_h_template = wiredtiger_h_template_path();

    // Generate wiredtiger_config.h
    let config_output = out_dir.join("wiredtiger_config.h");
    generate_config(&config_template, &config_output)
        .expect("Failed to generate wiredtiger_config.h");

    // Generate wiredtiger.h
    let wiredtiger_h_output = out_dir.join("wiredtiger.h");
    generate_wiredtiger_h(&wt_h_template, &wiredtiger_h_output)
        .expect("Failed to generate wiredtiger.h");

    gen_bindings(wiredtiger_h_output.as_path().to_str().unwrap());

    // Parse the filelist
    let files = parse_filelist(&filelist, &wt_root).expect("Failed to parse WiredTiger filelist");

    if files.is_empty() {
        panic!("No source files found in filelist");
    }

    // Build the WiredTiger library
    let mut build = cc::Build::new();

    // TODO - use build.flag_if_supported
    build.flags(["-Wno-unused-function"]);

    // Add include paths
    build.include(&out_dir); // For generated wiredtiger_config.h
    build.include(wt_root.join("src/include"));

    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    let is_darwin = os == "macos" || os == "ios";

    if is_darwin {
        build.include(wt_root.join("oss/apple"));
    }

    // Add all source files
    for file in &files {
        if file.exists() {
            build.file(file);
        } else {
            eprintln!("Warning: Source file not found: {}", file.display());
        }
    }

    // Compile the library
    build.compile("wt");

    // Tell cargo to rerun the build script if these files change
    println!("cargo:rerun-if-changed={}", filelist.display());
    println!("cargo:rerun-if-changed={}", config_template.display());
    println!("cargo:rerun-if-changed={}", wt_h_template.display());
}
