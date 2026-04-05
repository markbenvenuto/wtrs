use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Get the path to the WiredTiger source directory (git submodule at vendor/wiredtiger)
fn wiredtiger_root() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../vendor/wiredtiger")
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
        .replace("@VERSION_STRING@", VERSION_STRING)
        .replace(
            "#if !defined(SWIG) && !defined(DOXYGEN)",
            "#if !defined(BINDGEN_FILTER)",
        );

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
    let bindings = bindgen::Builder::default()
        .header(wiredtiger_h_output)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .clang_arg("-DBINDGEN_FILTER")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
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

    println!("cargo:rerun-if-changed={}", config_template.display());
    println!("cargo:rerun-if-changed={}", wt_h_template.display());
}
