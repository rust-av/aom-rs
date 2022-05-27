use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::thread::available_parallelism;
use std::{env, io};

fn format_write(builder: bindgen::Builder) -> String {
    builder
        .generate()
        .unwrap()
        .to_string()
        .replace("/**", "/*")
        .replace("/*!", "/*")
}

fn main() {
    let libs = if env::var("CARGO_FEATURE_BUILD_SOURCES").is_ok() {
        println!(
            "cargo:rustc-link-search=native={}",
            search().join("lib").to_string_lossy()
        );
        println!("cargo:rustc-link-lib=static=aom");
        if fs::metadata(&search().join("lib").join("libaom.a")).is_err() {
            fs::create_dir_all(&output()).expect("failed to create build directory");
            fetch().unwrap();
            build().unwrap();
        }

        env::set_var("SYSTEM_DEPS_LINK", "static");
        env::set_var("SYSTEM_DEPS_BUILD_INTERNAL", "always");
        system_deps::Config::new()
            .add_build_internal("aom", |lib, version| {
                // Actually build the library here
                system_deps::Library::from_internal_pkg_config(pkg_config(), lib, version)
            })
            .probe()
            .unwrap()
    } else {
        // Use system libraries
        system_deps::Config::new().probe().unwrap()
    };
    let headers = libs.all_include_paths();

    let mut builder = bindgen::builder()
        .header("data/aom.h")
        .blocklist_type("max_align_t")
        .size_t_is_usize(true)
        .default_enum_style(bindgen::EnumVariation::ModuleConsts);
    if env::var("CARGO_FEATURE_ACCOUNTING").is_ok() {
        builder = builder
            .header("data/accounting.h")
            .header("data/inspection.h");
    }

    for header in headers {
        builder = builder.clang_arg("-I").clang_arg(header.to_str().unwrap());
    }

    // Manually fix the comment so rustdoc won't try to pick them
    let s = format_write(builder);

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut file = File::create(out_path.join("aom.rs")).unwrap();

    let _ = file.write(s.as_bytes());
}

fn fetch() -> io::Result<()> {
    let output_base_path = output();
    let clone_dest_dir = format!("aom-{}", AOM_VERSION);
    let _ = std::fs::remove_dir_all(output_base_path.join(&clone_dest_dir));
    let status = Command::new("git")
        .current_dir(&output_base_path)
        .arg("clone")
        .arg("--depth=1")
        .arg("-b")
        .arg(format!("v{}", AOM_VERSION))
        .arg("https://aomedia.googlesource.com/aom")
        .arg(&clone_dest_dir)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "fetch failed"))
    }
}

fn build() -> io::Result<()> {
    let source_dir = source();

    let build_dir = "_build";
    let mut cmake = Command::new("cmake");
    cmake.current_dir(&source_dir);
    cmake
        .arg("-B")
        .arg(build_dir)
        .arg(format!(
            "-DCMAKE_BUILD_TYPE={}",
            if env::var("DEBUG").is_ok() {
                "Debug"
            } else {
                "Release"
            }
        ))
        .arg(format!(
            "-DCMAKE_INSTALL_PREFIX={}",
            search().to_string_lossy()
        ))
        .arg("-DCMAKE_INSTALL_LIBDIR=lib")
        .arg("-DCONFIG_AV1_DECODER=1")
        .arg("-DCONFIG_AV1_ENCODER=1")
        .arg("-DBUILD_SHARED_LIBS=1")
        .arg("-DENABLE_TESTS=0")
        .arg("-DENABLE_EXAMPLES=0")
        .arg("-DENABLE_DOCS=0");

    if env::var("CARGO_FEATURE_ACCOUNTING").is_ok() {
        // These features are needed for doing aomanalyzer-style analysis.
        cmake
            .arg("-DCONFIG_ACCOUNTING=1")
            .arg("-DCONFIG_INSPECTION=1");
    }

    cmake.arg("./");

    let output = cmake
        .output()
        .unwrap_or_else(|_| panic!("{:?} failed", cmake));
    if !output.status.success() {
        println!("cmake: {}", String::from_utf8_lossy(&output.stdout));

        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("cmake failed {}", String::from_utf8_lossy(&output.stderr)),
        ));
    }

    if !Command::new("make")
        .current_dir(&source())
        .arg("-C")
        .arg(build_dir)
        .arg("-j")
        .arg(available_parallelism().unwrap().to_string())
        .current_dir(&source())
        .status()?
        .success()
    {
        return Err(io::Error::new(io::ErrorKind::Other, "make failed"));
    }

    if !Command::new("make")
        .current_dir(&source())
        .arg("-C")
        .arg(build_dir)
        .arg("install")
        .status()?
        .success()
    {
        return Err(io::Error::new(io::ErrorKind::Other, "make install failed"));
    }

    Ok(())
}

fn output() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").unwrap())
}

const AOM_VERSION: &str = "3.3.0";

fn source() -> PathBuf {
    output().join(format!("aom-{}", AOM_VERSION))
}

fn search() -> PathBuf {
    let mut absolute = env::current_dir().unwrap();
    absolute.push(&output());
    absolute.push("dist");

    absolute
}

fn pkg_config() -> PathBuf {
    search().join("lib").join("pkgconfig")
}
