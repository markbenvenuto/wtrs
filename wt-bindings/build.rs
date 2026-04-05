use wt_build::{config_template_path, generate_config, generate_wiredtiger_h,
               wiredtiger_h_template_path};

use std::env;
use std::path::PathBuf;

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

    let config_output = out_dir.join("wiredtiger_config.h");
    generate_config(&config_template, &config_output)
        .expect("Failed to generate wiredtiger_config.h");

    let wiredtiger_h_output = out_dir.join("wiredtiger.h");
    generate_wiredtiger_h(&wt_h_template, &wiredtiger_h_output)
        .expect("Failed to generate wiredtiger.h");

    gen_bindings(wiredtiger_h_output.as_path().to_str().unwrap());

    println!("cargo:rerun-if-changed={}", config_template.display());
    println!("cargo:rerun-if-changed={}", wt_h_template.display());
}
