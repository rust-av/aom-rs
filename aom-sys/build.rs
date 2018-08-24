// build.rs

// extern crate cmake;
#[cfg(unix)]
extern crate bindgen;
extern crate cmake;
#[cfg(unix)]
extern crate pkg_config;

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let cargo_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let build_path = Path::new(&cargo_dir).join("data/aom");
    let out_dir = &Path::new(&cargo_dir).join("data/aom_build");

    if cfg!(feature = "build_sources") {
        let debug = if let Some(v) = env::var("PROFILE").ok() {
            match v.as_str() {
                "bench" | "release" => false,
                _ => true,
            }
        } else {
            false
        };

        let _ = cmake::Config::new(build_path)
            .define("CONFIG_DEBUG", (debug as u8).to_string())
            .define("CONFIG_ANALYZER", "0")
            .define("ENABLE_DOCS", "0")
            .define("ENABLE_TESTS", "0")
            .out_dir(out_dir)
            .no_build_target(cfg!(windows))
            .build();

        // Dirty hack to force a rebuild whenever the defaults are changed upstream
        let _ = fs::remove_file(out_dir.join("build/CMakeCache.txt"));
    }

    env::set_var("PKG_CONFIG_PATH", out_dir.join("lib/pkgconfig"));
    let _libs = pkg_config::Config::new().statik(true).probe("aom").unwrap();

    use std::io::Write;

    let headers = _libs.include_paths.clone();

    let mut builder = bindgen::builder()
        .blacklist_type("max_align_t")
        .rustfmt_bindings(false)
        .header("data/aom.h");

    for header in headers {
        builder = builder.clang_arg("-I").clang_arg(header.to_str().unwrap());
    }

    // Manually fix the comment so rustdoc won't try to pick them
    let s = builder
        .generate()
        .unwrap()
        .to_string()
        .replace("/**", "/*")
        .replace("/*!", "/*");

    let output = Path::new(&cargo_dir).join("src/aom.rs");

    use std::fs::OpenOptions;
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(output)
        .unwrap();

    let _ = file.write(s.as_bytes());
}
