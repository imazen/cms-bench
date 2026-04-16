fn main() {
    let icc_dir = std::path::Path::new("argyll-icc");

    cc::Build::new()
        .file(icc_dir.join("icc.c"))
        .file("argyll_glue.c")
        .include(icc_dir)
        .opt_level(2)
        .warnings(false)
        .compile("argyll_icc");
}
