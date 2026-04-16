//! Interleaved CMS transform benchmarks.
//!
//! Run: cargo bench
//! Save baseline: cargo bench -- --save-baseline=main
//! Compare: cargo bench -- --baseline=main

use cms_bench::*;
use std::path::Path;
use zenbench::prelude::*;

// ── skcms Send wrapper ─────────────────────────────────────────────────

struct SendProfile {
    _backing: Option<Vec<u8>>,
    profile: skcms_sys::skcms_ICCProfile,
}

unsafe impl Send for SendProfile {}
unsafe impl Sync for SendProfile {}

impl SendProfile {
    fn srgb() -> Self {
        let profile = unsafe {
            std::ptr::read(skcms_sys::srgb_profile() as *const skcms_sys::skcms_ICCProfile)
        };
        Self {
            _backing: None,
            profile,
        }
    }

    fn parse(data: &[u8]) -> Option<Self> {
        let backing = data.to_vec();
        let profile = skcms_sys::parse_icc_profile(&backing)?;
        Some(Self {
            _backing: Some(backing),
            profile,
        })
    }

    fn get(&self) -> &skcms_sys::skcms_ICCProfile {
        &self.profile
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn gen_u8(n: usize) -> Vec<u8> {
    (0..n * 3).map(|i| (i % 256) as u8).collect()
}

fn gen_u16(n: usize) -> Vec<u16> {
    (0..n * 3).map(|i| ((i * 257) % 65536) as u16).collect()
}

fn gen_f32(n: usize) -> Vec<f32> {
    (0..n * 3).map(|i| (i as f32) / (n * 3) as f32).collect()
}

// ── sRGB identity (baseline) ────────────────────────────────────────────

fn bench_srgb_identity(suite: &mut Suite) {
    for npix in [256usize, 4096, 65536] {
        suite.group(format!("sRGB_identity_u8_{npix}"), |g| {
            g.throughput(Throughput::Bytes((npix * 3) as u64));
            let input = gen_u8(npix);

            {
                let s = moxcms::ColorProfile::new_srgb();
                let xf = s
                    .create_transform_8bit(
                        moxcms::Layout::Rgb,
                        &s,
                        moxcms::Layout::Rgb,
                        moxcms::TransformOptions::default(),
                    )
                    .unwrap();
                let input = input.clone();
                g.bench("moxcms", move |b| {
                    let mut out = vec![0u8; npix * 3];
                    b.iter(|| {
                        xf.transform(black_box(&input), black_box(&mut out))
                            .unwrap();
                    })
                });
            }
            {
                let sp = SendProfile::srgb();
                let input = input.clone();
                g.bench("skcms", move |b| {
                    let mut out = vec![0u8; npix * 3];
                    b.iter(|| {
                        skcms_sys::transform(
                            black_box(&input),
                            skcms_sys::skcms_PixelFormat::RGB_888,
                            skcms_sys::skcms_AlphaFormat::Opaque,
                            sp.get(),
                            black_box(&mut out),
                            skcms_sys::skcms_PixelFormat::RGB_888,
                            skcms_sys::skcms_AlphaFormat::Opaque,
                            sp.get(),
                            npix,
                        );
                    })
                });
            }
            {
                let s = lcms2::Profile::new_srgb();
                let xf = lcms2::Transform::new(
                    &s,
                    lcms2::PixelFormat::RGB_8,
                    &s,
                    lcms2::PixelFormat::RGB_8,
                    lcms2::Intent::Perceptual,
                )
                .unwrap();
                let input = input.clone();
                g.bench("lcms2", move |b| {
                    let mut out = vec![0u8; npix * 3];
                    b.iter(|| {
                        xf.transform_pixels(black_box(&input), black_box(&mut out));
                    })
                });
            }
        });
    }
}

// ── Matrix profiles: src→sRGB u8 ────────────────────────────────────────

fn bench_matrix_u8(suite: &mut Suite) {
    let dir = matrix_dir();
    let npix = 65536usize;

    for (filename, label) in matrix_profiles() {
        let icc_data = match load_profile(&dir, filename) {
            Some(d) => d,
            None => continue,
        };

        suite.group(format!("{label}_u8_{npix}"), |g| {
            g.throughput(Throughput::Bytes((npix * 3) as u64));
            let input = gen_u8(npix);

            if let Ok(src) = moxcms::ColorProfile::new_from_slice(&icc_data) {
                let dst = moxcms::ColorProfile::new_srgb();
                if let Ok(xf) = src.create_transform_8bit(
                    moxcms::Layout::Rgb,
                    &dst,
                    moxcms::Layout::Rgb,
                    moxcms::TransformOptions::default(),
                ) {
                    let input = input.clone();
                    g.bench("moxcms", move |b| {
                        let mut out = vec![0u8; npix * 3];
                        b.iter(|| {
                            xf.transform(black_box(&input), black_box(&mut out))
                                .unwrap();
                        })
                    });
                }
            }

            if let Some(sp) = SendProfile::parse(&icc_data) {
                let srgb = SendProfile::srgb();
                let input = input.clone();
                g.bench("skcms", move |b| {
                    let mut out = vec![0u8; npix * 3];
                    b.iter(|| {
                        skcms_sys::transform(
                            black_box(&input),
                            skcms_sys::skcms_PixelFormat::RGB_888,
                            skcms_sys::skcms_AlphaFormat::Opaque,
                            sp.get(),
                            black_box(&mut out),
                            skcms_sys::skcms_PixelFormat::RGB_888,
                            skcms_sys::skcms_AlphaFormat::Opaque,
                            srgb.get(),
                            npix,
                        );
                    })
                });
            }

            if let Ok(src) = lcms2::Profile::new_icc(&icc_data) {
                let dst = lcms2::Profile::new_srgb();
                if let Ok(xf) = lcms2::Transform::new(
                    &src,
                    lcms2::PixelFormat::RGB_8,
                    &dst,
                    lcms2::PixelFormat::RGB_8,
                    lcms2::Intent::Perceptual,
                ) {
                    let input = input.clone();
                    g.bench("lcms2", move |b| {
                        let mut out = vec![0u8; npix * 3];
                        b.iter(|| {
                            xf.transform_pixels(black_box(&input), black_box(&mut out));
                        })
                    });
                }
            }
        });
    }
}

// ── LUT profiles: src→sRGB u8 ───────────────────────────────────────────

fn bench_lut_u8(suite: &mut Suite) {
    let dir = lut_dir();
    let npix = 65536usize;

    for (filename, label) in lut_profiles() {
        let icc_data = match load_profile(&dir, filename) {
            Some(d) => d,
            None => continue,
        };

        suite.group(format!("{label}_lut_u8_{npix}"), |g| {
            g.throughput(Throughput::Bytes((npix * 3) as u64));
            let input = gen_u8(npix);

            if let Ok(src) = moxcms::ColorProfile::new_from_slice(&icc_data) {
                let dst = moxcms::ColorProfile::new_srgb();
                if let Ok(xf) = src.create_transform_8bit(
                    moxcms::Layout::Rgb,
                    &dst,
                    moxcms::Layout::Rgb,
                    moxcms::TransformOptions::default(),
                ) {
                    let input = input.clone();
                    g.bench("moxcms", move |b| {
                        let mut out = vec![0u8; npix * 3];
                        b.iter(|| {
                            xf.transform(black_box(&input), black_box(&mut out))
                                .unwrap();
                        })
                    });
                }
            }

            if let Some(sp) = SendProfile::parse(&icc_data) {
                let srgb = SendProfile::srgb();
                let input = input.clone();
                g.bench("skcms", move |b| {
                    let mut out = vec![0u8; npix * 3];
                    b.iter(|| {
                        skcms_sys::transform(
                            black_box(&input),
                            skcms_sys::skcms_PixelFormat::RGB_888,
                            skcms_sys::skcms_AlphaFormat::Opaque,
                            sp.get(),
                            black_box(&mut out),
                            skcms_sys::skcms_PixelFormat::RGB_888,
                            skcms_sys::skcms_AlphaFormat::Opaque,
                            srgb.get(),
                            npix,
                        );
                    })
                });
            }

            if let Ok(src) = lcms2::Profile::new_icc(&icc_data) {
                let dst = lcms2::Profile::new_srgb();
                if let Ok(xf) = lcms2::Transform::new(
                    &src,
                    lcms2::PixelFormat::RGB_8,
                    &dst,
                    lcms2::PixelFormat::RGB_8,
                    lcms2::Intent::Perceptual,
                ) {
                    let input = input.clone();
                    g.bench("lcms2", move |b| {
                        let mut out = vec![0u8; npix * 3];
                        b.iter(|| {
                            xf.transform_pixels(black_box(&input), black_box(&mut out));
                        })
                    });
                }
            }
        });
    }
}

// ── u16 comparison at 65536px ────────────────────────────────────────────

fn bench_srgb_u16(suite: &mut Suite) {
    let npix = 65536usize;
    suite.group(format!("sRGB_identity_u16_{npix}"), |g| {
        g.throughput(Throughput::Bytes((npix * 6) as u64));
        let input = gen_u16(npix);

        {
            let s = moxcms::ColorProfile::new_srgb();
            let xf = s
                .create_transform_16bit(
                    moxcms::Layout::Rgb,
                    &s,
                    moxcms::Layout::Rgb,
                    moxcms::TransformOptions::default(),
                )
                .unwrap();
            let input = input.clone();
            g.bench("moxcms", move |b| {
                let mut out = vec![0u16; npix * 3];
                b.iter(|| {
                    xf.transform(black_box(&input), black_box(&mut out))
                        .unwrap();
                })
            });
        }
        {
            let sp = SendProfile::srgb();
            let input = input.clone();
            g.bench("skcms", move |b| {
                let mut out = vec![0u16; npix * 3];
                b.iter(|| {
                    skcms_sys::transform_u16(
                        black_box(&input),
                        skcms_sys::skcms_PixelFormat::RGB_161616LE,
                        skcms_sys::skcms_AlphaFormat::Opaque,
                        sp.get(),
                        black_box(&mut out),
                        skcms_sys::skcms_PixelFormat::RGB_161616LE,
                        skcms_sys::skcms_AlphaFormat::Opaque,
                        sp.get(),
                        npix,
                    );
                })
            });
        }
        {
            let s = lcms2::Profile::new_srgb();
            let xf: lcms2::Transform<[u16; 3], [u16; 3]> = lcms2::Transform::new(
                &s,
                lcms2::PixelFormat::RGB_16,
                &s,
                lcms2::PixelFormat::RGB_16,
                lcms2::Intent::Perceptual,
            )
            .unwrap();
            let input = input.clone();
            g.bench("lcms2", move |b| {
                let src: Vec<[u16; 3]> =
                    input.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
                let mut dst = vec![[0u16; 3]; npix];
                b.iter(|| {
                    xf.transform_pixels(black_box(&src), black_box(&mut dst));
                })
            });
        }
    });
}

// ── f32 comparison at 65536px ────────────────────────────────────────────

fn bench_srgb_f32(suite: &mut Suite) {
    let npix = 65536usize;
    suite.group(format!("sRGB_identity_f32_{npix}"), |g| {
        g.throughput(Throughput::Bytes((npix * 12) as u64));
        let input = gen_f32(npix);

        {
            let s = moxcms::ColorProfile::new_srgb();
            let xf = s
                .create_transform_f32(
                    moxcms::Layout::Rgb,
                    &s,
                    moxcms::Layout::Rgb,
                    moxcms::TransformOptions::default(),
                )
                .unwrap();
            let input = input.clone();
            g.bench("moxcms", move |b| {
                let mut out = vec![0f32; npix * 3];
                b.iter(|| {
                    xf.transform(black_box(&input), black_box(&mut out))
                        .unwrap();
                })
            });
        }
        {
            let sp = SendProfile::srgb();
            let input = input.clone();
            g.bench("skcms", move |b| {
                let mut out = vec![0f32; npix * 3];
                b.iter(|| {
                    skcms_sys::transform_f32(
                        black_box(&input),
                        skcms_sys::skcms_PixelFormat::RGB_fff,
                        skcms_sys::skcms_AlphaFormat::Opaque,
                        sp.get(),
                        black_box(&mut out),
                        skcms_sys::skcms_PixelFormat::RGB_fff,
                        skcms_sys::skcms_AlphaFormat::Opaque,
                        sp.get(),
                        npix,
                    );
                })
            });
        }
    });
}

zenbench::main!(
    bench_srgb_identity,
    bench_matrix_u8,
    bench_lut_u8,
    bench_srgb_u16,
    bench_srgb_f32,
);
