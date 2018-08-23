extern crate bindgen;
extern crate metadeps;

#[cfg(feature="build")]
extern crate cmake;

use std::fs::OpenOptions;
use std::io::Write;

fn format_write(builder: bindgen::Builder, output: &str) {
    let s = builder
        .generate()
        .unwrap()
        .to_string()
        .replace("/**", "/*")
        .replace("/*!", "/*");

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(output)
        .unwrap();

    let _ = file.write(s.as_bytes());
}

fn common_builder() -> bindgen::Builder {
    bindgen::builder()
        .raw_line("#![allow(dead_code)]")
        .raw_line("#![allow(non_camel_case_types)]")
        .raw_line("#![allow(non_snake_case)]")
        .raw_line("#![allow(non_upper_case_globals)]")
}


#[cfg(feature="build")]
fn build_sources() {
    use cmake::Config;
    use std::env;
    use std::path::Path;
    use std::process::Command;

    let build_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    Command::new("git")
        .args(&["submodule", "update", "--recommend-shallow", "--init"])
        .spawn()
        .expect("Update failed");

    let build_path = Path::new(&build_dir).join("data/aom");

    let cfg = Config::new(build_path).build();

    env::set_var("PKG_CONFIG_PATH", cfg.join("lib/pkgconfig"));
}

#[cfg(not(feature="build"))]
fn build_sources() {}

fn main() {
    if cfg!(feature="build") {
        build_sources()
    }

    let libs = metadeps::probe().unwrap();
    let headers = libs.get("aom").unwrap().include_paths.clone();
    // let buildver = libs.get("vpx").unwrap().version.split(".").nth(1).unwrap();

    let mut builder = common_builder()
        .header("data/aom.h")
        .blacklist_type("max_align_t"); // https://github.com/rust-lang-nursery/rust-bindgen/issues/550

    for header in headers {
        builder = builder.clang_arg("-I").clang_arg(header.to_str().unwrap());
    }

    // Manually fix the comment so rustdoc won't try to pick them
    format_write(builder, "src/aom.rs");
}
