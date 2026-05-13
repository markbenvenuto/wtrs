use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// Version info
pub const VERSION_MAJOR: &str = "12";
pub const VERSION_MINOR: &str = "0";
pub const VERSION_PATCH: &str = "0";
pub const VERSION_STRING: &str = "\"WiredTiger 12.0.0 (Rust build)\"";

/// Get the path to the WiredTiger source directory (git submodule at vendor/wiredtiger)
pub fn wiredtiger_root() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../vendor/wiredtiger")
}

pub fn filelist_path() -> PathBuf {
    wiredtiger_root().join("dist/filelist")
}

pub fn config_template_path() -> PathBuf {
    wiredtiger_root().join("cmake/configs/wiredtiger_config.h.in")
}

pub fn wiredtiger_h_template_path() -> PathBuf {
    wiredtiger_root().join("src/include/wiredtiger.h.in")
}

/// Determine the current CPU architecture filter
pub fn get_arch_filter() -> &'static str {
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
pub fn get_os_filter() -> &'static str {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match os.as_str() {
        "macos" | "ios" => "DARWIN_HOST",
        "linux" | "android" => "LINUX_HOST",
        "windows" => "WINDOWS_HOST",
        _ => "",
    }
}

/// Check if we're building for a POSIX-compatible system
pub fn is_posix() -> bool {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    matches!(
        os.as_str(),
        "macos" | "ios" | "linux" | "android" | "freebsd" | "openbsd" | "netbsd" | "dragonfly"
    )
}

/// Check if a file with a given filter should be included in the build
pub fn should_include_file(
    filter: &str,
    arch_filter: &str,
    os_filter: &str,
    is_posix: bool,
) -> bool {
    if filter.is_empty() {
        return true;
    }
    match filter {
        "X86_HOST" | "ARM64_HOST" | "POWERPC_HOST" | "RISCV64_HOST" | "LOONGARCH64_HOST"
        | "ZSERIES_HOST" => filter == arch_filter,
        "DARWIN_HOST" | "LINUX_HOST" | "WINDOWS_HOST" => filter == os_filter,
        "POSIX_HOST" => is_posix,
        _ => {
            eprintln!("Warning: Unknown filter '{}', excluding file", filter);
            false
        }
    }
}

/// Parse the filelist and return a list of source files applicable to the current platform
pub fn parse_filelist(filelist_path: &Path, wt_root: &Path) -> Result<Vec<PathBuf>, String> {
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

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

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

        if should_include_file(filter, arch_filter, os_filter, posix) {
            let full_path = wt_root.join(file_path);
            files.push(full_path);
        }
    }

    Ok(files)
}

/// Get the set of enabled features based on the target platform
pub fn get_enabled_features() -> HashMap<&'static str, bool> {
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
pub fn get_spinlock_type() -> &'static str {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match os.as_str() {
        "macos" | "ios" | "linux" | "android" => "SPINLOCK_PTHREAD_MUTEX",
        "windows" => "SPINLOCK_MSVC",
        _ => "SPINLOCK_PTHREAD_MUTEX",
    }
}

/// Process a #cmakedefine line
pub fn process_cmakedefine(line: &str, features: &HashMap<&str, bool>) -> String {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return format!("/* {} */", line);
    }

    let symbol = parts[1];
    let value = parts.get(2).copied();
    let is_enabled = features.get(symbol).copied().unwrap_or(false);

    match value {
        Some("0") => {
            // CMake semantics: #cmakedefine VAR 0 generates /* #undef VAR */ when VAR is disabled,
            // not #define VAR 0. Without this fix, `defined(HAVE_RCPC)` would be true on
            // non-ARM platforms, causing ARM-specific assembly to be emitted.
            if is_enabled {
                format!("#define {} 0", symbol)
            } else {
                format!("/* #undef {} */", symbol)
            }
        }
        Some("1") => {
            if is_enabled {
                format!("#define {} 1", symbol)
            } else {
                format!("/* #undef {} */", symbol)
            }
        }
        Some(v) if v.starts_with('@') => {
            if is_enabled {
                format!("#define {} {}", symbol, v)
            } else {
                format!("/* #undef {} */", symbol)
            }
        }
        None => {
            if is_enabled {
                format!("#define {}", symbol)
            } else {
                format!("/* #undef {} */", symbol)
            }
        }
        Some(v) => {
            if is_enabled {
                format!("#define {} {}", symbol, v)
            } else {
                format!("/* #undef {} */", symbol)
            }
        }
    }
}

/// Generate wiredtiger_config.h from the template
pub fn generate_config(template_path: &Path, output_path: &Path) -> Result<(), String> {
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
            let mut processed = line.to_string();
            processed = processed.replace("@VERSION_MAJOR@", VERSION_MAJOR);
            processed = processed.replace("@VERSION_MINOR@", VERSION_MINOR);
            processed = processed.replace("@VERSION_PATCH@", VERSION_PATCH);
            processed = processed.replace("@SPINLOCK_TYPE_CONFIG_VAR@", spinlock_type);
            output_lines.push(processed);
        }
    }

    fs::write(output_path, output_lines.join("\n")).map_err(|e| {
        format!(
            "Failed to write config file '{}': {}",
            output_path.display(),
            e
        )
    })
}

/// Generate wiredtiger.h from the template
pub fn generate_wiredtiger_h(template_path: &Path, output_path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(template_path).map_err(|e| {
        format!(
            "Failed to read wiredtiger.h template '{}': {}",
            template_path.display(),
            e
        )
    })?;

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
    })
}
