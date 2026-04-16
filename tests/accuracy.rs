//! Accuracy comparison across CMS engines.
//!
//! For each ICC profile × intent × config, transforms a u16 ramp through
//! each engine, then reports pairwise max/mean channel differences.
//!
//! Columns include self-comparisons:
//!   - mox_def↔hq: moxcms default vs HQ (measures fixed-point quantization)
//!   - lcms2_def↔hq: lcms2 default vs HQ (measures pipeline optimization cost)
//!
//! Run: cargo test --release -- --nocapture

use cms_bench::*;
use std::path::Path;

fn diff(a: &Option<Vec<u16>>, b: &Option<Vec<u16>>) -> Option<(u32, f64)> {
    match (a, b) {
        (Some(a), Some(b)) => Some((max_channel_diff(a, b), mean_channel_diff(a, b))),
        _ => None,
    }
}

fn fmt_d(d: Option<(u32, f64)>) -> String {
    match d {
        Some((max, mean)) => format!("{max:>5} ({mean:>6.1})"),
        None => "    —         ".into(),
    }
}

fn run_table(dir: &Path, profiles: &[(&str, &str)], header: &str) {
    let ramp = make_ramp_u16();

    for intent in [Intent::Perceptual, Intent::RelativeColorimetric] {
        for config in [Config::Default, Config::HighQuality] {
            eprintln!("\n── {header} | intent={intent} config={config} ──");
            eprintln!(
                "{:<16} {:>14} {:>14} {:>14} {:>14} {:>14} {:>14}",
                "Profile",
                "mox↔lcms2",
                "mox↔skcms",
                "lcms2↔skcms",
                "argyll↔lcms2",
                "mox_def↔hq",
                "lcms2_def↔hq",
            );
            eprintln!("{:-<112}", "");

            for &(filename, label) in profiles {
                let data = match load_profile(dir, filename) {
                    Some(d) => d,
                    None => continue,
                };

                let mox = moxcms_transform_u16(&data, &ramp, config, intent);
                let lcm = lcms2_transform_u16(&data, &ramp, config, intent);
                let skc = skcms_transform_u16(&data, &ramp, intent);
                let arg = argyll_transform_u16(&data, &ramp, intent);

                // Self-comparisons: each engine's default vs HQ
                let mox_other = moxcms_transform_u16(
                    &data,
                    &ramp,
                    match config {
                        Config::Default => Config::HighQuality,
                        Config::HighQuality => Config::Default,
                    },
                    intent,
                );
                let lcm_other = lcms2_transform_u16(
                    &data,
                    &ramp,
                    match config {
                        Config::Default => Config::HighQuality,
                        Config::HighQuality => Config::Default,
                    },
                    intent,
                );

                eprintln!(
                    "{:<16} {:>14} {:>14} {:>14} {:>14} {:>14} {:>14}",
                    label,
                    fmt_d(diff(&mox, &lcm)),
                    fmt_d(diff(&mox, &skc)),
                    fmt_d(diff(&lcm, &skc)),
                    fmt_d(diff(&arg, &lcm)),
                    fmt_d(diff(&mox, &mox_other)),
                    fmt_d(diff(&lcm, &lcm_other)),
                );
            }
        }
    }
}

#[test]
fn accuracy_matrix_profiles() {
    eprintln!(
        "\n{:=<120}",
        "═ Matrix-shaper accuracy (max u16 / mean u16, 320-pixel ramp) "
    );
    run_table(&matrix_dir(), &matrix_profiles(), "matrix");
}

#[test]
fn accuracy_lut_profiles() {
    eprintln!(
        "\n{:=<120}",
        "═ LUT-based accuracy (max u16 / mean u16, 320-pixel ramp) "
    );
    run_table(&lut_dir(), &lut_profiles(), "LUT");
}
