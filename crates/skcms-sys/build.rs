use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let skcms_dir = manifest_dir.join("skcms");
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    // Check if skcms source exists
    if !skcms_dir.join("skcms.cc").exists() {
        panic!(
            "skcms source not found at {:?}. \
            Please run: git clone https://skia.googlesource.com/skcms {:?}",
            skcms_dir, skcms_dir
        );
    }

    // Base compilation settings
    let mut base = cc::Build::new();
    base.cpp(true)
        .include(&skcms_dir)
        .flag_if_supported("-std=c++17")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-function")
        .flag_if_supported("-fno-exceptions")
        .flag_if_supported("-fno-rtti")
        .opt_level(3);

    // Compile main skcms.cc with baseline settings
    base.clone()
        .file(skcms_dir.join("skcms.cc"))
        .file(skcms_dir.join("src/skcms_TransformBaseline.cc"))
        .compile("skcms_base");

    // x86/x86_64-specific SIMD variants
    if target_arch == "x86_64" || target_arch == "x86" {
        // Compile HSW (Haswell) variant with AVX2/F16C
        base.clone()
            .file(skcms_dir.join("src/skcms_TransformHsw.cc"))
            .flag("-march=haswell")
            .compile("skcms_hsw");

        // Compile SKX (Skylake-X) variant with AVX-512
        base.clone()
            .file(skcms_dir.join("src/skcms_TransformSkx.cc"))
            .flag("-march=skylake-avx512")
            .compile("skcms_skx");
    }

    println!("cargo:rerun-if-changed={}", skcms_dir.display());
}
