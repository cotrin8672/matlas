use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=MATLABROOT");
    println!("cargo:rerun-if-changed=native/mat_shim.c");
    println!("cargo:rerun-if-changed=native/mex_entry.c");
    if env::var_os("DOCS_RS").is_some() {
        println!("cargo:rustc-env=MATRUST_BUILD_RELEASE=0");
        return;
    }
    let root = PathBuf::from(
        env::var_os("MATLABROOT").expect("Set MATLABROOT to your MATLAB installation."),
    );
    let include = root.join("extern/include");
    for name in ["mat.h", "matrix.h", "mex.h", "tmwtypes.h"] {
        let header = include.join(name);
        assert!(
            header.is_file(),
            "Missing MATLAB header: {}",
            header.display()
        );
        println!("cargo:rerun-if-changed={}", header.display());
    }
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let (lib, prefix) = match (os.as_str(), arch.as_str()) {
        ("windows", "x86_64") => {
            assert!(
                !env::var("CARGO_CFG_TARGET_FEATURE")
                    .unwrap_or_default()
                    .split(',')
                    .any(|f| f == "crt-static"),
                "MATLAB FILE* diagnostics require the shared MSVC CRT; crt-static is unsupported"
            );
            assert_eq!(
                env::var("CARGO_CFG_TARGET_ENV").unwrap(),
                "msvc",
                "Use the MSVC Rust target"
            );
            (root.join("extern/lib/win64/microsoft"), "lib")
        }
        ("linux", "x86_64") => (root.join("bin/glnxa64"), ""),
        ("macos", "aarch64") => (root.join("bin/maca64"), ""),
        ("macos", "x86_64") => (root.join("bin/maci64"), ""),
        _ => panic!("Unsupported MATLAB target: {os}/{arch}"),
    };
    assert!(
        lib.is_dir(),
        "Missing MATLAB library directory: {}",
        lib.display()
    );
    let version_file = root.join("VersionInfo.xml");
    println!("cargo:rerun-if-changed={}", version_file.display());
    let version = std::fs::read_to_string(&version_file).expect("Read MATLAB VersionInfo.xml");
    let release = version
        .split("<release>R")
        .nth(1)
        .and_then(|s| s.split('<').next())
        .expect("MATLAB release identifier");
    let release = u32::from_str_radix(release, 16).expect("MATLAB release must look like R2025a");
    println!("cargo:rustc-env=MATRUST_BUILD_RELEASE={release}");
    cc::Build::new()
        .static_crt(false)
        .file("native/mat_shim.c")
        .file("native/mex_entry.c")
        .include(include)
        .define("TARGET_API_VERSION", "800")
        .define("MATRUST_BUILD_RELEASE", release.to_string().as_str())
        .warnings(true)
        .compile("matlas800");
    println!("cargo:rustc-link-search=native={}", lib.display());
    for name in ["mat", "mx", "mex"] {
        println!("cargo:rustc-link-lib={prefix}{name}");
    }
}
