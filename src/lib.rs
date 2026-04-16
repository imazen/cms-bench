//! CMS transform wrappers for benchmarking and accuracy comparison.
//!
//! Each engine exposes transforms in two configurations:
//! - **Default**: production settings (what users get out of the box)
//! - **HQ**: maximum accuracy (float, no optimization, high precision)
//!
//! And two rendering intents: Perceptual and Relative Colorimetric.

use std::path::{Path, PathBuf};

// ── Profile loading ─────────────────────────────────────────────────────

pub fn profiles_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles")
}

pub fn matrix_dir() -> PathBuf {
    profiles_dir().join("matrix")
}

pub fn lut_dir() -> PathBuf {
    profiles_dir().join("lut")
}

/// All matrix-shaper profiles with short labels for display.
pub fn matrix_profiles() -> Vec<(&'static str, &'static str)> {
    vec![
        ("DisplayP3Compat-v2-micro.icc", "P3_v2"),
        ("Rec2020Compat-v2-micro.icc", "Rec2020_v2"),
        ("AdobeCompat-v2.icc", "AdobeRGB_v2"),
        ("ProPhoto-v2-micro.icc", "ProPhoto_v2"),
        ("WideGamutCompat-v2.icc", "WideGamut_v2"),
        ("Rec709-v2-micro.icc", "Rec709_v2"),
        ("Rec601PAL-v2-micro.icc", "Rec601PAL_v2"),
        ("AppleCompat-v2.icc", "AppleRGB_v2"),
        ("ColorMatchCompat-v2.icc", "ColorMatch_v2"),
        ("DisplayP3-v4.icc", "P3_v4"),
        ("Rec2020-v4.icc", "Rec2020_v4"),
        ("ProPhoto-v4.icc", "ProPhoto_v4"),
        ("moxcms_display_p3.icc", "moxP3"),
        ("moxcms_bt.2020.icc", "moxBT2020"),
        ("AdobeRGB1998-sys.icc", "sysAdobeRGB"),
        ("moxcms_bt.2020_pq.icc", "BT2020_PQ"),
        ("moxcms_display_p3_pq.icc", "P3_PQ"),
        ("moxcms_bt.2020_hlg.icc", "BT2020_HLG"),
    ]
}

/// LUT-based profiles with short labels.
pub fn lut_profiles() -> Vec<(&'static str, &'static str)> {
    vec![
        ("AdobeCS4-RGB-VideoHD.icc", "VideoHD"),
        ("AdobeCS4-RGB-VideoPAL.icc", "VideoPAL"),
        ("Kodak_sRGB.icc", "KodakSRGB"),
    ]
}

pub fn load_profile(dir: &Path, name: &str) -> Option<Vec<u8>> {
    std::fs::read(dir.join(name)).ok()
}

// ── Test ramp ───────────────────────────────────────────────────────────

pub const RAMP_PIXELS: usize = 320; // 64 gray + 3×64 per-channel + 64 mixed

pub fn make_ramp_u8() -> Vec<u8> {
    make_ramp_u16().iter().map(|&v| (v >> 8) as u8).collect()
}

pub fn make_ramp_u16() -> Vec<u16> {
    let steps = 64usize;
    let mut px = Vec::with_capacity(RAMP_PIXELS * 3);
    for i in 0..steps {
        let v = ((i as f64 / (steps - 1) as f64) * 65535.0) as u16;
        px.extend_from_slice(&[v, v, v]);
    }
    for ch in 0..3usize {
        for i in 0..steps {
            let v = ((i as f64 / (steps - 1) as f64) * 65535.0) as u16;
            let mut p = [0u16; 3];
            p[ch] = v;
            px.extend_from_slice(&p);
        }
    }
    for i in 0..steps {
        let t = i as f64 / (steps - 1) as f64;
        let r = ((t * 0.9 + 0.05) * 65535.0) as u16;
        let g = (((1.0 - t) * 0.8 + 0.1) * 65535.0) as u16;
        let b = ((((t * 2.0) % 1.0) * 0.7 + 0.15) * 65535.0) as u16;
        px.extend_from_slice(&[r, g, b]);
    }
    px
}

// ── Diff helpers ────────────────────────────────────────────────────────

pub fn max_channel_diff(a: &[u16], b: &[u16]) -> u32 {
    a.iter()
        .zip(b)
        .map(|(&x, &y)| (x as i32 - y as i32).unsigned_abs())
        .max()
        .unwrap_or(0)
}

pub fn mean_channel_diff(a: &[u16], b: &[u16]) -> f64 {
    if a.is_empty() {
        return 0.0;
    }
    let sum: u64 = a
        .iter()
        .zip(b)
        .map(|(&x, &y)| (x as i64 - y as i64).unsigned_abs())
        .sum();
    sum as f64 / a.len() as f64
}

// ── Intent + Configuration ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Perceptual,
    RelativeColorimetric,
}

impl std::fmt::Display for Intent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Intent::Perceptual => f.write_str("perc"),
            Intent::RelativeColorimetric => f.write_str("relcol"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Config {
    Default,
    HighQuality,
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Config::Default => f.write_str("default"),
            Config::HighQuality => f.write_str("hq"),
        }
    }
}

// ── moxcms transforms ───────────────────────────────────────────────────

pub fn moxcms_transform_u16(
    icc_data: &[u8],
    ramp: &[u16],
    config: Config,
    intent: Intent,
) -> Option<Vec<u16>> {
    use moxcms::*;

    let src = ColorProfile::new_from_slice(icc_data).ok()?;
    let dst = ColorProfile::new_srgb();

    let ri = match intent {
        Intent::Perceptual => RenderingIntent::Perceptual,
        Intent::RelativeColorimetric => RenderingIntent::RelativeColorimetric,
    };

    let opts = match config {
        Config::Default => TransformOptions {
            rendering_intent: ri,
            ..TransformOptions::default()
        },
        Config::HighQuality => TransformOptions {
            rendering_intent: ri,
            allow_use_cicp_transfer: false,
            prefer_fixed_point: false,
            interpolation_method: InterpolationMethod::Tetrahedral,
            barycentric_weight_scale: BarycentricWeightScale::High,
        },
    };

    let t = src
        .create_transform_16bit(Layout::Rgb, &dst, Layout::Rgb, opts)
        .ok()?;
    let mut out = vec![0u16; ramp.len()];
    t.transform(ramp, &mut out).ok()?;
    Some(out)
}

pub fn moxcms_transform_u8(
    icc_data: &[u8],
    input: &[u8],
    npix: usize,
    config: Config,
    intent: Intent,
) -> Option<Vec<u8>> {
    use moxcms::*;

    let src = ColorProfile::new_from_slice(icc_data).ok()?;
    let dst = ColorProfile::new_srgb();

    let ri = match intent {
        Intent::Perceptual => RenderingIntent::Perceptual,
        Intent::RelativeColorimetric => RenderingIntent::RelativeColorimetric,
    };

    let opts = match config {
        Config::Default => TransformOptions {
            rendering_intent: ri,
            ..TransformOptions::default()
        },
        Config::HighQuality => TransformOptions {
            rendering_intent: ri,
            allow_use_cicp_transfer: false,
            prefer_fixed_point: false,
            interpolation_method: InterpolationMethod::Tetrahedral,
            barycentric_weight_scale: BarycentricWeightScale::High,
        },
    };

    let t = src
        .create_transform_8bit(Layout::Rgb, &dst, Layout::Rgb, opts)
        .ok()?;
    let mut out = vec![0u8; npix * 3];
    t.transform(input, &mut out).ok()?;
    Some(out)
}

// ── lcms2 transforms ────────────────────────────────────────────────────

pub fn lcms2_transform_u16(
    icc_data: &[u8],
    ramp: &[u16],
    config: Config,
    intent: Intent,
) -> Option<Vec<u16>> {
    use lcms2::{Flags, Intent as LIntent, PixelFormat, Profile, Transform};

    let src = Profile::new_icc(icc_data).ok()?;
    let dst = Profile::new_srgb();

    let li = match intent {
        Intent::Perceptual => LIntent::Perceptual,
        Intent::RelativeColorimetric => LIntent::RelativeColorimetric,
    };

    let xform: Transform<[u16; 3], [u16; 3]> = match config {
        Config::Default => {
            Transform::new(&src, PixelFormat::RGB_16, &dst, PixelFormat::RGB_16, li).ok()?
        }
        Config::HighQuality => {
            let flags = Flags::NO_OPTIMIZE | Flags::HIGHRES_PRECALC;
            Transform::new_flags(
                &src,
                PixelFormat::RGB_16,
                &dst,
                PixelFormat::RGB_16,
                li,
                flags,
            )
            .ok()?
        }
    };

    let mut out = vec![0u16; ramp.len()];
    let src_px: &[[u16; 3]] = bytemuck::cast_slice(ramp);
    let dst_px: &mut [[u16; 3]] = bytemuck::cast_slice_mut(&mut out);
    xform.transform_pixels(src_px, dst_px);
    Some(out)
}

// ── skcms transforms ────────────────────────────────────────────────────

/// skcms has no quality knobs. Intent is controlled via A2B parse priority.
pub fn skcms_transform_u16(icc_data: &[u8], ramp: &[u16], intent: Intent) -> Option<Vec<u16>> {
    use skcms_sys::*;

    let priority: &[i32] = match intent {
        Intent::Perceptual => &[0, 1],
        Intent::RelativeColorimetric => &[1, 0],
    };

    let profile = parse_icc_profile_with_priority(icc_data, priority)?;
    let srgb = srgb_profile();
    let npix = ramp.len() / 3;
    let mut out = vec![0u16; ramp.len()];

    let ok = transform_u16(
        ramp,
        skcms_PixelFormat::RGB_161616LE,
        skcms_AlphaFormat::Opaque,
        &profile,
        &mut out,
        skcms_PixelFormat::RGB_161616LE,
        skcms_AlphaFormat::Opaque,
        srgb,
        npix,
    );
    if ok { Some(out) } else { None }
}

// ── ArgyllCMS transforms ────────────────────────────────────────────────

/// ArgyllCMS has no quality knobs. V2 profiles only.
pub fn argyll_transform_u16(icc_data: &[u8], ramp: &[u16], intent: Intent) -> Option<Vec<u16>> {
    let ai = match intent {
        Intent::Perceptual => argyll_sys::Intent::Perceptual,
        Intent::RelativeColorimetric => argyll_sys::Intent::RelativeColorimetric,
    };
    let npix = ramp.len() / 3;
    let mut out = vec![0u16; ramp.len()];
    let ok = argyll_sys::transform_u16(icc_data, argyll_sys::SRGB_ICC, ai, ramp, &mut out, npix);
    if ok { Some(out) } else { None }
}
