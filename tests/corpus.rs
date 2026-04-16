//! Corpus-wide accuracy test (issue #3).
//!
//! Scans `profiles/corpus/` (363 ICC profiles deduplicated from the
//! zenpixels R2 manifest by normalized FNV-1a hash), transforms a u16
//! ramp through each CMS engine, and reports:
//!   - Parse pass/fail per engine per profile
//!   - Transform pass/fail per engine per profile
//!   - Pairwise max/mean channel deltas (moxcms ↔ lcms2 ↔ skcms ↔ argyll)
//!   - Aggregate histogram of max diffs
//!   - Writes a TSV to /tmp/cms-bench-corpus.tsv
//!
//! Run: cargo test --release --test corpus -- --nocapture

use cms_bench::*;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

// ── Per-profile run ─────────────────────────────────────────────────────

struct RunResult {
    #[allow(dead_code)]
    color_space: String,
    size_bytes: usize,
    mox_def: Option<Vec<u16>>,
    mox_hq: Option<Vec<u16>>,
    lcm_def: Option<Vec<u16>>,
    lcm_hq: Option<Vec<u16>>,
    skc: Option<Vec<u16>>,
    arg: Option<Vec<u16>>,
}

fn run_profile(dir: &Path, name: &str, intent: Intent) -> RunResult {
    let data = std::fs::read(dir.join(name)).unwrap_or_default();
    let cs = if data.len() >= 20 {
        std::str::from_utf8(&data[16..20])
            .unwrap_or("????")
            .to_string()
    } else {
        "????".to_string()
    };
    let size = data.len();

    // Only RGB profiles can go through our RGB-in-RGB-out pipeline.
    // For non-RGB, we record parsing results but skip transforms.
    let is_rgb = &data[16..20.min(data.len())] == b"RGB ";
    let ramp = make_ramp_u16();

    if !is_rgb {
        return RunResult {
            color_space: cs,
            size_bytes: size,
            mox_def: None,
            mox_hq: None,
            lcm_def: None,
            lcm_hq: None,
            skc: None,
            arg: None,
        };
    }

    RunResult {
        color_space: cs,
        size_bytes: size,
        mox_def: moxcms_transform_u16(&data, &ramp, Config::Default, intent),
        mox_hq: moxcms_transform_u16(&data, &ramp, Config::HighQuality, intent),
        lcm_def: lcms2_transform_u16(&data, &ramp, Config::Default, intent),
        lcm_hq: lcms2_transform_u16(&data, &ramp, Config::HighQuality, intent),
        skc: skcms_transform_u16(&data, &ramp, intent),
        arg: argyll_transform_u16(&data, &ramp, intent),
    }
}

fn diff(a: &Option<Vec<u16>>, b: &Option<Vec<u16>>) -> Option<u32> {
    match (a, b) {
        (Some(a), Some(b)) => Some(max_diff_u16(a, b)),
        _ => None,
    }
}

fn histogram(name: &str, vals: &[u32]) {
    if vals.is_empty() {
        eprintln!("  {name:<24}  (no data)");
        return;
    }
    let n = vals.len();
    let bucket = |lo: u32, hi: u32| vals.iter().filter(|&&v| v >= lo && v <= hi).count();
    let max = *vals.iter().max().unwrap();
    let sum: u64 = vals.iter().map(|&v| v as u64).sum();
    let mean = sum as f64 / n as f64;

    let mut sorted = vals.to_vec();
    sorted.sort();
    let median = sorted[sorted.len() / 2];
    let p95 = sorted[sorted.len() * 95 / 100];
    let p99 = sorted[sorted.len() * 99 / 100];

    eprintln!(
        "  {name:<24}  n={n:>4}  =0={:>4}  ≤4={:>4}  ≤16={:>4}  ≤64={:>4}  ≤256={:>4}  ≤1024={:>4}  >1024={:>4}  max={max:>5}  mean={mean:>7.1}  med={median:>5}  p95={p95:>5}  p99={p99:>5}",
        bucket(0, 0),
        bucket(0, 4),
        bucket(0, 16),
        bucket(0, 64),
        bucket(0, 256),
        bucket(0, 1024),
        bucket(1025, u32::MAX),
    );
}

#[test]
fn corpus_accuracy() {
    let dir = corpus_dir();
    let names = list_corpus(&dir);
    assert!(
        !names.is_empty(),
        "No profiles in {} — did you forget to populate the corpus?",
        dir.display()
    );
    eprintln!("\nCorpus: {} profiles from {}", names.len(), dir.display());

    // Track parse/transform success counts per engine
    let mut parse_ok: BTreeMap<&str, usize> = BTreeMap::new();
    let mut xform_ok: BTreeMap<&str, usize> = BTreeMap::new();
    let mut cs_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut non_rgb_skipped = 0usize;

    // Collect pairwise diffs per intent
    let mut diffs_perc: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
    let mut diffs_relcol: BTreeMap<&str, Vec<u32>> = BTreeMap::new();

    // Top-10 worst per pair, per intent
    let mut worst_perc: BTreeMap<&str, Vec<(u32, String)>> = BTreeMap::new();
    let mut worst_relcol: BTreeMap<&str, Vec<(u32, String)>> = BTreeMap::new();

    let tsv_path = Path::new("/tmp/cms-bench-corpus.tsv");
    let mut tsv = std::io::BufWriter::new(std::fs::File::create(tsv_path).expect("create tsv"));
    writeln!(
        tsv,
        "profile\tcs\tsize\tintent\tmox_def_lcm_def\tmox_hq_lcm_hq\tmox_def_skc\tmox_hq_skc\t\
         lcm_def_skc\tlcm_hq_skc\targ_lcm_hq\tmox_def_mox_hq\tlcm_def_lcm_hq"
    )
    .unwrap();

    for (i, name) in names.iter().enumerate() {
        if i % 50 == 0 {
            eprintln!("  [{}/{}] {name}", i + 1, names.len());
        }

        // Parse each engine once to record parse success.
        // (The transform_u16 wrappers parse internally; we duplicate here to track failures.)
        let data = std::fs::read(dir.join(name)).unwrap_or_default();
        if data.len() < 132 {
            continue;
        }
        let cs = std::str::from_utf8(&data[16..20])
            .unwrap_or("????")
            .to_string();
        *cs_counts.entry(cs.clone()).or_default() += 1;

        let mox_ok = moxcms::ColorProfile::new_from_slice(&data).is_ok();
        let lcm_ok = lcms2::Profile::new_icc(&data).is_ok();
        let skc_ok = skcms_sys::parse_icc_profile(&data).is_some();

        if mox_ok {
            *parse_ok.entry("moxcms").or_default() += 1;
        }
        if lcm_ok {
            *parse_ok.entry("lcms2").or_default() += 1;
        }
        if skc_ok {
            *parse_ok.entry("skcms").or_default() += 1;
        }

        if cs != "RGB " {
            non_rgb_skipped += 1;
            continue;
        }

        for intent in [Intent::Perceptual, Intent::RelativeColorimetric] {
            let r = run_profile(&dir, name, intent);

            if r.mox_def.is_some() {
                *xform_ok.entry("moxcms_def").or_default() += 1;
            }
            if r.lcm_def.is_some() {
                *xform_ok.entry("lcms2_def").or_default() += 1;
            }
            if r.skc.is_some() {
                *xform_ok.entry("skcms").or_default() += 1;
            }
            if r.arg.is_some() {
                *xform_ok.entry("argyll").or_default() += 1;
            }

            let pairs: &[(&str, Option<u32>)] = &[
                ("mox_def↔lcm_def", diff(&r.mox_def, &r.lcm_def)),
                ("mox_hq↔lcm_hq", diff(&r.mox_hq, &r.lcm_hq)),
                ("mox_def↔skc", diff(&r.mox_def, &r.skc)),
                ("mox_hq↔skc", diff(&r.mox_hq, &r.skc)),
                ("lcm_def↔skc", diff(&r.lcm_def, &r.skc)),
                ("lcm_hq↔skc", diff(&r.lcm_hq, &r.skc)),
                ("arg↔lcm_hq", diff(&r.arg, &r.lcm_hq)),
                ("mox_def↔mox_hq", diff(&r.mox_def, &r.mox_hq)),
                ("lcm_def↔lcm_hq", diff(&r.lcm_def, &r.lcm_hq)),
            ];

            let target = if intent == Intent::Perceptual {
                &mut diffs_perc
            } else {
                &mut diffs_relcol
            };
            let worst = if intent == Intent::Perceptual {
                &mut worst_perc
            } else {
                &mut worst_relcol
            };
            for (pair_name, d) in pairs {
                if let Some(v) = *d {
                    target.entry(pair_name).or_default().push(v);
                    let entry = worst.entry(pair_name).or_default();
                    entry.push((v, name.clone()));
                    entry.sort_by(|a, b| b.0.cmp(&a.0));
                    entry.truncate(10);
                }
            }

            // TSV row
            let fmt = |d: Option<u32>| d.map(|v| v.to_string()).unwrap_or_else(|| "NA".into());
            writeln!(
                tsv,
                "{name}\t{cs}\t{}\t{intent}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                r.size_bytes,
                fmt(diff(&r.mox_def, &r.lcm_def)),
                fmt(diff(&r.mox_hq, &r.lcm_hq)),
                fmt(diff(&r.mox_def, &r.skc)),
                fmt(diff(&r.mox_hq, &r.skc)),
                fmt(diff(&r.lcm_def, &r.skc)),
                fmt(diff(&r.lcm_hq, &r.skc)),
                fmt(diff(&r.arg, &r.lcm_hq)),
                fmt(diff(&r.mox_def, &r.mox_hq)),
                fmt(diff(&r.lcm_def, &r.lcm_hq)),
            )
            .unwrap();
        }
    }

    drop(tsv);

    // ── Summary ──

    eprintln!("\n══ Parse pass counts ══");
    for (engine, count) in &parse_ok {
        eprintln!("  {engine}: {count}/{}", names.len());
    }
    eprintln!("\n══ Color space distribution ══");
    for (cs, count) in &cs_counts {
        eprintln!("  {cs:?}: {count}");
    }
    eprintln!("  Non-RGB skipped for transforms: {non_rgb_skipped}");

    eprintln!("\n══ Transform pass counts (RGB profiles × 2 intents) ══");
    for (engine, count) in &xform_ok {
        eprintln!("  {engine}: {count}");
    }

    for (intent, diffs, worst) in [
        ("Perceptual", &diffs_perc, &worst_perc),
        ("RelCol", &diffs_relcol, &worst_relcol),
    ] {
        eprintln!("\n══ {intent} intent: pairwise max u16 diffs ══");
        for (pair, vals) in diffs {
            histogram(pair, vals);
        }

        eprintln!("\n══ {intent} intent: top-5 worst profiles per pair ══");
        for (pair, entries) in worst {
            eprintln!("  {pair}:");
            for (d, n) in entries.iter().take(5) {
                eprintln!("    {d:>5} u16  {n}");
            }
        }
    }

    eprintln!("\nTSV written to {}", tsv_path.display());
}
