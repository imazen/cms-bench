//! Accuracy comparison across CMS engines in Default and HQ configurations.
//!
//! For each ICC profile, transforms a u16 ramp through each engine at both
//! config levels, then reports pairwise max/mean channel differences.
//!
//! Run: cargo test --release -- --nocapture

use cms_bench::*;
use std::path::Path;

struct Row {
    label: &'static str,
    config: Config,
    mox_vs_lcms2: Option<(u32, f64)>,
    mox_vs_skcms: Option<(u32, f64)>,
    lcms2_vs_skcms: Option<(u32, f64)>,
    argyll_vs_lcms2: Option<(u32, f64)>,
    mox_def_vs_hq: Option<(u32, f64)>,
}

fn diff(a: &Option<Vec<u16>>, b: &Option<Vec<u16>>) -> Option<(u32, f64)> {
    match (a, b) {
        (Some(a), Some(b)) => Some((max_channel_diff(a, b), mean_channel_diff(a, b))),
        _ => None,
    }
}

fn run_profile(dir: &Path, filename: &str, label: &'static str, config: Config) -> Row {
    let data = load_profile(dir, filename).expect(filename);
    let ramp = make_ramp_u16();

    let mox = moxcms_transform_u16(&data, &ramp, config);
    let lcm = lcms2_transform_u16(&data, &ramp, config);
    let skc = skcms_transform_u16(&data, &ramp);
    let arg = argyll_transform_u16(&data, &ramp);

    // For mox default vs HQ, we need both
    let mox_other = if config == Config::Default {
        moxcms_transform_u16(&data, &ramp, Config::HighQuality)
    } else {
        moxcms_transform_u16(&data, &ramp, Config::Default)
    };

    Row {
        label,
        config,
        mox_vs_lcms2: diff(&mox, &lcm),
        mox_vs_skcms: diff(&mox, &skc),
        lcms2_vs_skcms: diff(&lcm, &skc),
        argyll_vs_lcms2: diff(&arg, &lcm),
        mox_def_vs_hq: diff(&mox, &mox_other),
    }
}

fn fmt_d(d: Option<(u32, f64)>) -> String {
    match d {
        Some((max, mean)) => format!("{max:>5} ({mean:>5.1})"),
        None => "    —       ".into(),
    }
}

#[test]
fn accuracy_matrix_profiles() {
    let dir = matrix_dir();
    let profiles = matrix_profiles();

    eprintln!(
        "\n{:=<120}",
        "═ Matrix-shaper accuracy (max u16 / mean u16, perceptual intent, 320-pixel ramp) "
    );
    for config in [Config::Default, Config::HighQuality] {
        eprintln!("\n── Config: {config} ──");
        eprintln!(
            "{:<16} {:>16} {:>16} {:>16} {:>16} {:>16}",
            "Profile", "mox↔lcms2", "mox↔skcms", "lcms2↔skcms", "argyll↔lcms2", "mox_def↔hq"
        );
        eprintln!("{:-<112}", "");

        for &(filename, label) in &profiles {
            let row = run_profile(&dir, filename, label, config);
            eprintln!(
                "{:<16} {:>16} {:>16} {:>16} {:>16} {:>16}",
                row.label,
                fmt_d(row.mox_vs_lcms2),
                fmt_d(row.mox_vs_skcms),
                fmt_d(row.lcms2_vs_skcms),
                fmt_d(row.argyll_vs_lcms2),
                fmt_d(row.mox_def_vs_hq),
            );
        }
    }
}

#[test]
fn accuracy_lut_profiles() {
    let dir = lut_dir();
    let profiles = lut_profiles();

    eprintln!(
        "\n{:=<120}",
        "═ LUT-based accuracy (max u16 / mean u16, perceptual intent, 320-pixel ramp) "
    );
    for config in [Config::Default, Config::HighQuality] {
        eprintln!("\n── Config: {config} ──");
        eprintln!(
            "{:<16} {:>16} {:>16} {:>16} {:>16} {:>16}",
            "Profile", "mox↔lcms2", "mox↔skcms", "lcms2↔skcms", "argyll↔lcms2", "mox_def↔hq"
        );
        eprintln!("{:-<112}", "");

        for &(filename, label) in &profiles {
            let row = run_profile(&dir, filename, label, config);
            eprintln!(
                "{:<16} {:>16} {:>16} {:>16} {:>16} {:>16}",
                row.label,
                fmt_d(row.mox_vs_lcms2),
                fmt_d(row.mox_vs_skcms),
                fmt_d(row.lcms2_vs_skcms),
                fmt_d(row.argyll_vs_lcms2),
                fmt_d(row.mox_def_vs_hq),
            );
        }
    }
}
