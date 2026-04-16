//! FFI bindings to ArgyllCMS icclib for ICC profile transforms.
//!
//! Provides a single high-level function [`transform_u16`] that converts
//! RGB u16 pixels from a source ICC profile to a destination profile
//! using ArgyllCMS's icclib lookup engine.

use std::os::raw::{c_int, c_uchar, c_ushort};

unsafe extern "C" {
    fn argyll_transform_u16(
        src_icc_data: *const c_uchar,
        src_icc_len: usize,
        dst_icc_data: *const c_uchar,
        dst_icc_len: usize,
        intent: c_int,
        src_pixels: *const c_ushort,
        dst_pixels: *mut c_ushort,
        npixels: usize,
    ) -> c_int;
}

/// Rendering intent for ArgyllCMS transforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Intent {
    Perceptual = 0,
    RelativeColorimetric = 1,
    Saturation = 2,
    AbsoluteColorimetric = 3,
}

/// Transform interleaved RGB u16 pixels from `src_icc` profile to `dst_icc` profile.
///
/// `src_pixels` and `dst_pixels` are slices of `npixels * 3` u16 values.
/// Returns `true` on success, `false` if ArgyllCMS failed to parse or transform.
pub fn transform_u16(
    src_icc: &[u8],
    dst_icc: &[u8],
    intent: Intent,
    src_pixels: &[u16],
    dst_pixels: &mut [u16],
    npixels: usize,
) -> bool {
    assert!(src_pixels.len() >= npixels * 3);
    assert!(dst_pixels.len() >= npixels * 3);

    let rv = unsafe {
        argyll_transform_u16(
            src_icc.as_ptr(),
            src_icc.len(),
            dst_icc.as_ptr(),
            dst_icc.len(),
            intent as c_int,
            src_pixels.as_ptr(),
            dst_pixels.as_mut_ptr(),
            npixels,
        )
    };
    rv == 0
}

/// Built-in sRGB ICC profile bytes (from ArgyllCMS's bundled sRGB.icm).
///
/// We embed the profile so callers don't need to locate it at runtime.
pub static SRGB_ICC: &[u8] = include_bytes!("../sRGB.icm");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb_identity() {
        // sRGB -> sRGB should be near-identity
        let src = [32768u16, 16384, 49152];
        let mut dst = [0u16; 3];

        let ok = transform_u16(SRGB_ICC, SRGB_ICC, Intent::Perceptual, &src, &mut dst, 1);
        assert!(ok, "ArgyllCMS sRGB identity transform failed");

        // Should be very close to input (within a few u16 of rounding)
        for i in 0..3 {
            let diff = (src[i] as i32 - dst[i] as i32).unsigned_abs();
            assert!(
                diff < 256,
                "channel {i}: src={} dst={} diff={diff}",
                src[i],
                dst[i]
            );
        }
    }
}
