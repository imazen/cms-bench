//! FFI bindings to Google's skcms color management library
//!
//! skcms is a small, fast color management library used by Chrome/Skia.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::c_void;

// Matrix types
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct skcms_Matrix3x3 {
    pub vals: [[f32; 3]; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct skcms_Matrix3x4 {
    pub vals: [[f32; 4]; 3],
}

// Transfer function
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct skcms_TransferFunction {
    pub g: f32,
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum skcms_TFType {
    Invalid = 0,
    sRGBish = 1,
    PQish = 2,
    HLGish = 3,
    HLGinvish = 4,
    PQ = 5,
    HLG = 6,
}

// Curve (union in C, we represent as a struct with the larger variant)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct skcms_Curve {
    pub table_entries: u32,
    pub parametric_or_table_8: skcms_TransferFunction, // overlaps with table pointers
}

// A2B and B2A structures
#[repr(C)]
#[derive(Clone, Copy)]
pub struct skcms_A2B {
    pub input_curves: [skcms_Curve; 4],
    pub grid_8: *const u8,
    pub grid_16: *const u8,
    pub input_channels: u32,
    pub grid_points: [u8; 4],
    pub matrix_curves: [skcms_Curve; 3],
    pub matrix: skcms_Matrix3x4,
    pub matrix_channels: u32,
    pub output_channels: u32,
    pub output_curves: [skcms_Curve; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct skcms_B2A {
    pub input_curves: [skcms_Curve; 3],
    pub input_channels: u32,
    pub matrix_channels: u32,
    pub matrix_curves: [skcms_Curve; 3],
    pub matrix: skcms_Matrix3x4,
    pub output_curves: [skcms_Curve; 4],
    pub grid_8: *const u8,
    pub grid_16: *const u8,
    pub grid_points: [u8; 4],
    pub output_channels: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct skcms_CICP {
    pub color_primaries: u8,
    pub transfer_characteristics: u8,
    pub matrix_coefficients: u8,
    pub video_full_range_flag: u8,
}

// ICC Profile
#[repr(C)]
pub struct skcms_ICCProfile {
    pub buffer: *const u8,
    pub size: u32,
    pub data_color_space: u32,
    pub pcs: u32,
    pub tag_count: u32,
    pub trc: [skcms_Curve; 3],
    pub toXYZD50: skcms_Matrix3x3,
    pub A2B: skcms_A2B,
    pub B2A: skcms_B2A,
    pub CICP: skcms_CICP,
    pub has_trc: bool,
    pub has_toXYZD50: bool,
    pub has_A2B: bool,
    pub has_B2A: bool,
    pub has_CICP: bool,
}

// Signatures
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum skcms_Signature {
    CMYK = 0x434D594B,
    Gray = 0x47524159,
    RGB = 0x52474220,
    Lab = 0x4C616220,
    XYZ = 0x58595A20,
}

// Pixel formats
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum skcms_PixelFormat {
    A_8 = 0,
    A_8_ = 1,
    G_8 = 2,
    G_8_ = 3,
    GA_88 = 4,
    GA_88_ = 5,
    RGB_565 = 6,
    BGR_565 = 7,
    ABGR_4444 = 8,
    ARGB_4444 = 9,
    RGB_888 = 10,
    BGR_888 = 11,
    RGBA_8888 = 12,
    BGRA_8888 = 13,
    RGBA_8888_sRGB = 14,
    BGRA_8888_sRGB = 15,
    RGBA_1010102 = 16,
    BGRA_1010102 = 17,
    RGB_161616LE = 18,
    BGR_161616LE = 19,
    RGBA_16161616LE = 20,
    BGRA_16161616LE = 21,
    RGB_161616BE = 22,
    BGR_161616BE = 23,
    RGBA_16161616BE = 24,
    BGRA_16161616BE = 25,
    RGB_hhh_Norm = 26,
    BGR_hhh_Norm = 27,
    RGBA_hhhh_Norm = 28,
    BGRA_hhhh_Norm = 29,
    RGB_hhh = 30,
    BGR_hhh = 31,
    RGBA_hhhh = 32,
    BGRA_hhhh = 33,
    RGB_fff = 34,
    BGR_fff = 35,
    RGBA_ffff = 36,
    BGRA_ffff = 37,
    RGB_101010x_XR = 38,
    BGR_101010x_XR = 39,
    RGBA_10101010_XR = 40,
    BGRA_10101010_XR = 41,
}

// Alpha formats
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum skcms_AlphaFormat {
    Opaque = 0,
    Unpremul = 1,
    PremulAsEncoded = 2,
}

unsafe extern "C" {
    // Matrix operations
    pub fn skcms_Matrix3x3_invert(src: *const skcms_Matrix3x3, dst: *mut skcms_Matrix3x3) -> bool;
    pub fn skcms_Matrix3x3_concat(
        a: *const skcms_Matrix3x3,
        b: *const skcms_Matrix3x3,
    ) -> skcms_Matrix3x3;

    // Transfer function operations
    pub fn skcms_TransferFunction_eval(tf: *const skcms_TransferFunction, x: f32) -> f32;
    pub fn skcms_TransferFunction_invert(
        src: *const skcms_TransferFunction,
        dst: *mut skcms_TransferFunction,
    ) -> bool;
    pub fn skcms_TransferFunction_getType(tf: *const skcms_TransferFunction) -> skcms_TFType;

    // Profile functions
    pub fn skcms_sRGB_profile() -> *const skcms_ICCProfile;
    pub fn skcms_XYZD50_profile() -> *const skcms_ICCProfile;
    pub fn skcms_sRGB_TransferFunction() -> *const skcms_TransferFunction;

    // Profile parsing
    pub fn skcms_ParseWithA2BPriority(
        buf: *const c_void,
        len: usize,
        priority: *const i32,
        priorities: i32,
        profile: *mut skcms_ICCProfile,
    ) -> bool;

    // Profile comparison
    pub fn skcms_ApproximatelyEqualProfiles(
        a: *const skcms_ICCProfile,
        b: *const skcms_ICCProfile,
    ) -> bool;

    // Transform
    pub fn skcms_Transform(
        src: *const c_void,
        src_fmt: skcms_PixelFormat,
        src_alpha: skcms_AlphaFormat,
        src_profile: *const skcms_ICCProfile,
        dst: *mut c_void,
        dst_fmt: skcms_PixelFormat,
        dst_alpha: skcms_AlphaFormat,
        dst_profile: *const skcms_ICCProfile,
        npixels: usize,
    ) -> bool;

    // Profile utilities
    pub fn skcms_MakeUsableAsDestination(profile: *mut skcms_ICCProfile) -> bool;
    pub fn skcms_GetInputChannelCount(profile: *const skcms_ICCProfile) -> i32;
}

/// Parse an ICC profile from bytes
///
/// Returns true if parsing succeeded.
/// The buffer must remain valid for the lifetime of the profile.
///
/// # Safety
///
/// - `buf` must be a valid pointer to at least `len` bytes of ICC profile data
/// - `profile` must be a valid pointer to an uninitialized `skcms_ICCProfile`
/// - The buffer must remain valid for the lifetime of the profile
#[inline]
pub unsafe fn skcms_Parse(buf: *const c_void, len: usize, profile: *mut skcms_ICCProfile) -> bool {
    let priority = [0i32, 1i32];
    unsafe { skcms_ParseWithA2BPriority(buf, len, priority.as_ptr(), 2, profile) }
}

/// Safe wrapper for parsing ICC profiles
pub fn parse_icc_profile(data: &[u8]) -> Option<skcms_ICCProfile> {
    unsafe {
        let mut profile: skcms_ICCProfile = std::mem::zeroed();
        if skcms_Parse(data.as_ptr() as *const c_void, data.len(), &mut profile) {
            Some(profile)
        } else {
            None
        }
    }
}

/// Parse an ICC profile with explicit A2B table priority.
///
/// `priority` is a slice of A2B table indices (0=perceptual, 1=relative
/// colorimetric, 2=saturation). skcms will use the first available table
/// matching the priority order. This is skcms's mechanism for rendering
/// intent selection.
///
/// Examples:
/// - `&[0, 1]` — prefer perceptual (default skcms_Parse behavior)
/// - `&[1, 0]` — prefer relative colorimetric
pub fn parse_icc_profile_with_priority(data: &[u8], priority: &[i32]) -> Option<skcms_ICCProfile> {
    unsafe {
        let mut profile: skcms_ICCProfile = std::mem::zeroed();
        if skcms_ParseWithA2BPriority(
            data.as_ptr() as *const c_void,
            data.len(),
            priority.as_ptr(),
            priority.len() as i32,
            &mut profile,
        ) {
            Some(profile)
        } else {
            None
        }
    }
}

/// Get the built-in sRGB profile
pub fn srgb_profile() -> &'static skcms_ICCProfile {
    unsafe { &*skcms_sRGB_profile() }
}

/// Transform pixels between color profiles (u8 version)
#[allow(clippy::too_many_arguments)]
pub fn transform(
    src: &[u8],
    src_fmt: skcms_PixelFormat,
    src_alpha: skcms_AlphaFormat,
    src_profile: &skcms_ICCProfile,
    dst: &mut [u8],
    dst_fmt: skcms_PixelFormat,
    dst_alpha: skcms_AlphaFormat,
    dst_profile: &skcms_ICCProfile,
    npixels: usize,
) -> bool {
    unsafe {
        skcms_Transform(
            src.as_ptr() as *const c_void,
            src_fmt,
            src_alpha,
            src_profile,
            dst.as_mut_ptr() as *mut c_void,
            dst_fmt,
            dst_alpha,
            dst_profile,
            npixels,
        )
    }
}

/// Transform pixels between color profiles (u16 version)
#[allow(clippy::too_many_arguments)]
pub fn transform_u16(
    src: &[u16],
    src_fmt: skcms_PixelFormat,
    src_alpha: skcms_AlphaFormat,
    src_profile: &skcms_ICCProfile,
    dst: &mut [u16],
    dst_fmt: skcms_PixelFormat,
    dst_alpha: skcms_AlphaFormat,
    dst_profile: &skcms_ICCProfile,
    npixels: usize,
) -> bool {
    unsafe {
        skcms_Transform(
            src.as_ptr() as *const c_void,
            src_fmt,
            src_alpha,
            src_profile,
            dst.as_mut_ptr() as *mut c_void,
            dst_fmt,
            dst_alpha,
            dst_profile,
            npixels,
        )
    }
}

/// Transform pixels between color profiles (f32 version)
#[allow(clippy::too_many_arguments)]
pub fn transform_f32(
    src: &[f32],
    src_fmt: skcms_PixelFormat,
    src_alpha: skcms_AlphaFormat,
    src_profile: &skcms_ICCProfile,
    dst: &mut [f32],
    dst_fmt: skcms_PixelFormat,
    dst_alpha: skcms_AlphaFormat,
    dst_profile: &skcms_ICCProfile,
    npixels: usize,
) -> bool {
    unsafe {
        skcms_Transform(
            src.as_ptr() as *const c_void,
            src_fmt,
            src_alpha,
            src_profile,
            dst.as_mut_ptr() as *mut c_void,
            dst_fmt,
            dst_alpha,
            dst_profile,
            npixels,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb_profile() {
        let srgb = srgb_profile();
        assert!(srgb.has_trc);
        assert!(srgb.has_toXYZD50);
    }

    #[test]
    fn test_parse_and_transform() {
        // Test identity transform with sRGB
        let srgb = srgb_profile();

        let src = [128u8, 64, 192];
        let mut dst = [0u8; 3];

        let ok = transform(
            &src,
            skcms_PixelFormat::RGB_888,
            skcms_AlphaFormat::Opaque,
            srgb,
            &mut dst,
            skcms_PixelFormat::RGB_888,
            skcms_AlphaFormat::Opaque,
            srgb,
            1,
        );

        assert!(ok);
        assert_eq!(src, dst);
    }
}
