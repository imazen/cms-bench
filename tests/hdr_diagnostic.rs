use cms_bench::*;

#[test]
fn hdr_pixel_diagnostic() {
    let ramp = make_ramp_u16();
    let dir = matrix_dir();

    for (filename, label) in [
        ("moxcms_bt.2020_pq.icc", "BT2020_PQ"),
        ("moxcms_bt.2020_hlg.icc", "BT2020_HLG"),
    ] {
        let data = load_profile(&dir, filename).unwrap();

        eprintln!("\n=== {label} ===");

        for config in [Config::Default, Config::HighQuality] {
            for intent in [Intent::Perceptual, Intent::RelativeColorimetric] {
                let mox = moxcms_transform_u16(&data, &ramp, config, intent);
                let lcm = lcms2_transform_u16(&data, &ramp, config, intent);
                let skc = skcms_transform_u16(&data, &ramp, intent);
                let arg = argyll_transform_u16(&data, &ramp, intent);

                eprintln!("\n  config={config} intent={intent}");
                eprintln!(
                    "  {:>6} {:>8} {:>8} {:>8} {:>8}",
                    "input", "moxcms", "lcms2", "skcms", "argyll"
                );
                for i in [0, 4, 8, 16, 32, 48, 63] {
                    let inp = ramp[i * 3];
                    let m: i64 = mox.as_ref().map(|v| v[i * 3] as i64).unwrap_or(-1);
                    let l: i64 = lcm.as_ref().map(|v| v[i * 3] as i64).unwrap_or(-1);
                    let s: i64 = skc.as_ref().map(|v| v[i * 3] as i64).unwrap_or(-1);
                    let a: i64 = arg.as_ref().map(|v| v[i * 3] as i64).unwrap_or(-1);
                    eprintln!("  {:>6} {:>8} {:>8} {:>8} {:>8}", inp, m, l, s, a);
                }
            }
        }
    }
}
