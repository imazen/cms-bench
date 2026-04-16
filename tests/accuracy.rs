//! Accuracy comparison across CMS engines at u8 and u16.
//!
//! For each ICC profile × intent × config × bit depth, transforms a ramp
//! through each engine and reports pairwise max/mean channel differences.
//!
//! Self-comparison columns:
//!   - mox_def↔hq: moxcms default vs HQ (fixed-point quantization cost)
//!   - lcms2_def↔hq: lcms2 default vs HQ (pipeline optimization cost)
//!
//! Flags: when a backend has no config knob (skcms, argyll) or when
//! default==HQ for a given profile, that column shows "=def" or "=hq"
//! to indicate duplicate output was suppressed.
//!
//! Run: cargo test --release -- --nocapture

use cms_bench::*;
use std::path::Path;

// ── u16 diff helper ─────────────────────────────────────────────────────

fn diff16(a: &Option<Vec<u16>>, b: &Option<Vec<u16>>) -> Option<(u32, f64)> {
    match (a, b) {
        (Some(a), Some(b)) => Some((max_diff_u16(a, b), mean_diff_u16(a, b))),
        _ => None,
    }
}

fn diff8(a: &Option<Vec<u8>>, b: &Option<Vec<u8>>) -> Option<(u32, f64)> {
    match (a, b) {
        (Some(a), Some(b)) => Some((max_diff_u8(a, b), mean_diff_u8(a, b))),
        _ => None,
    }
}

fn fmt_d(d: Option<(u32, f64)>) -> String {
    match d {
        Some((0, _)) => "    0         ".into(),
        Some((max, mean)) => format!("{max:>5} ({mean:>6.1})"),
        None => "    —         ".into(),
    }
}

fn fmt_self(d: Option<(u32, f64)>, tag: &str) -> String {
    match d {
        Some((0, _)) => format!("   ={tag:<10}"),
        Some((max, mean)) => format!("{max:>5} ({mean:>6.1})"),
        None => "    —         ".into(),
    }
}

// ── Core test runner ────────────────────────────────────────────────────

fn run_table_u16(dir: &Path, profiles: &[(&str, &str)], header: &str) {
    let ramp = make_ramp_u16();

    for intent in [Intent::Perceptual, Intent::RelativeColorimetric] {
        // Run both configs, detect if they're identical per-engine
        eprintln!("\n── {header} u16 | intent={intent} ──");
        eprintln!(
            "{:<16} {:>14} {:>14} {:>14} {:>14} | {:>14} {:>14} {:>14} {:>14} | {:>14} {:>14}",
            "Profile",
            "mox↔lcms2 def",
            "mox↔skcms def",
            "lcms2↔skc def",
            "argyll↔lc def",
            "mox↔lcms2 hq",
            "mox↔skcms hq",
            "lcms2↔skc hq",
            "argyll↔lc hq",
            "mox_def↔hq",
            "lcms2_def↔hq",
        );
        eprintln!("{:-<180}", "");

        for &(filename, label) in profiles {
            let data = match load_profile(dir, filename) {
                Some(d) => d,
                None => continue,
            };

            let mox_def = moxcms_transform_u16(&data, &ramp, Config::Default, intent);
            let mox_hq = moxcms_transform_u16(&data, &ramp, Config::HighQuality, intent);
            let lcm_def = lcms2_transform_u16(&data, &ramp, Config::Default, intent);
            let lcm_hq = lcms2_transform_u16(&data, &ramp, Config::HighQuality, intent);
            let skc = skcms_transform_u16(&data, &ramp, intent);
            let arg = argyll_transform_u16(&data, &ramp, intent);

            let mox_self = diff16(&mox_def, &mox_hq);
            let lcm_self = diff16(&lcm_def, &lcm_hq);

            eprintln!(
                "{:<16} {:>14} {:>14} {:>14} {:>14} | {:>14} {:>14} {:>14} {:>14} | {:>14} {:>14}",
                label,
                fmt_d(diff16(&mox_def, &lcm_def)),
                fmt_d(diff16(&mox_def, &skc)),
                fmt_d(diff16(&lcm_def, &skc)),
                fmt_d(diff16(&arg, &lcm_def)),
                fmt_d(diff16(&mox_hq, &lcm_hq)),
                fmt_d(diff16(&mox_hq, &skc)),
                fmt_d(diff16(&lcm_hq, &skc)),
                fmt_d(diff16(&arg, &lcm_hq)),
                fmt_self(mox_self, "hq"),
                fmt_self(lcm_self, "hq"),
            );
        }
    }
}

fn run_table_u8(dir: &Path, profiles: &[(&str, &str)], header: &str) {
    let ramp = make_ramp_u8();
    let npix = RAMP_PIXELS;

    for intent in [Intent::Perceptual, Intent::RelativeColorimetric] {
        eprintln!("\n── {header} u8 | intent={intent} ──");
        eprintln!(
            "{:<16} {:>14} {:>14} {:>14} | {:>14} {:>14} {:>14} | {:>14} {:>14}",
            "Profile",
            "mox↔lcms2 def",
            "mox↔skcms def",
            "lcms2↔skc def",
            "mox↔lcms2 hq",
            "mox↔skcms hq",
            "lcms2↔skc hq",
            "mox_def↔hq",
            "lcms2_def↔hq",
        );
        eprintln!("{:-<148}", "");

        for &(filename, label) in profiles {
            let data = match load_profile(dir, filename) {
                Some(d) => d,
                None => continue,
            };

            let mox_def = moxcms_transform_u8(&data, &ramp, npix, Config::Default, intent);
            let mox_hq = moxcms_transform_u8(&data, &ramp, npix, Config::HighQuality, intent);
            let lcm_def = lcms2_transform_u8(&data, &ramp, npix, Config::Default, intent);
            let lcm_hq = lcms2_transform_u8(&data, &ramp, npix, Config::HighQuality, intent);
            let skc = skcms_transform_u8(&data, &ramp, npix, intent);

            let mox_self = diff8(&mox_def, &mox_hq);
            let lcm_self = diff8(&lcm_def, &lcm_hq);

            eprintln!(
                "{:<16} {:>14} {:>14} {:>14} | {:>14} {:>14} {:>14} | {:>14} {:>14}",
                label,
                fmt_d(diff8(&mox_def, &lcm_def)),
                fmt_d(diff8(&mox_def, &skc)),
                fmt_d(diff8(&lcm_def, &skc)),
                fmt_d(diff8(&mox_hq, &lcm_hq)),
                fmt_d(diff8(&mox_hq, &skc)),
                fmt_d(diff8(&lcm_hq, &skc)),
                fmt_self(mox_self, "hq"),
                fmt_self(lcm_self, "hq"),
            );
        }
    }
}

// ── Test entry points ───────────────────────────────────────────────────

#[test]
fn accuracy_matrix_u16() {
    eprintln!(
        "\n{:=<180}",
        "═ Matrix-shaper accuracy u16 (max / mean, 320-pixel ramp) "
    );
    run_table_u16(&matrix_dir(), &matrix_profiles(), "matrix");
}

#[test]
fn accuracy_matrix_u8() {
    eprintln!(
        "\n{:=<148}",
        "═ Matrix-shaper accuracy u8 (max / mean, 320-pixel ramp) "
    );
    run_table_u8(&matrix_dir(), &matrix_profiles(), "matrix");
}

#[test]
fn accuracy_lut_u16() {
    eprintln!(
        "\n{:=<180}",
        "═ LUT-based accuracy u16 (max / mean, 320-pixel ramp) "
    );
    run_table_u16(&lut_dir(), &lut_profiles(), "LUT");
}

#[test]
fn accuracy_lut_u8() {
    eprintln!(
        "\n{:=<148}",
        "═ LUT-based accuracy u8 (max / mean, 320-pixel ramp) "
    );
    run_table_u8(&lut_dir(), &lut_profiles(), "LUT");
}
