/*
 * Thin C glue around ArgyllCMS icclib for Rust FFI.
 *
 * Exposes a flat API:
 *   argyll_transform_u16(icc_data, icc_len, srgb_data, srgb_len,
 *                        intent, src_u16, dst_u16, npixels) -> int
 *
 * Converts npixels of RGB u16 data from the source ICC profile to the
 * destination ICC profile using ArgyllCMS's icclib lookup engine.
 *
 * Returns 0 on success, nonzero on failure.
 */

#include <stdlib.h>
#include <string.h>
#include "icc.h"

/* Intent constants matching Rust side */
#define ARGYLL_INTENT_PERCEPTUAL           0
#define ARGYLL_INTENT_RELATIVE_COLORIMETRIC 1
#define ARGYLL_INTENT_SATURATION           2
#define ARGYLL_INTENT_ABSOLUTE_COLORIMETRIC 3

static icRenderingIntent map_intent(int intent) {
    switch (intent) {
        case ARGYLL_INTENT_PERCEPTUAL:            return icPerceptual;
        case ARGYLL_INTENT_RELATIVE_COLORIMETRIC: return icRelativeColorimetric;
        case ARGYLL_INTENT_SATURATION:            return icSaturation;
        case ARGYLL_INTENT_ABSOLUTE_COLORIMETRIC: return icAbsoluteColorimetric;
        default:                                  return icPerceptual;
    }
}

/*
 * Transform npixels of interleaved RGB u16 data from src_profile to dst_profile.
 *
 * Two-stage transform through XYZ PCS:
 *   src_profile forward (device RGB -> XYZ)
 *   dst_profile backward (XYZ -> device RGB)
 *
 * IMPORTANT: on a backward (icmBwd) luobj, call lookup_fwd to go PCS->device.
 * The "fwd/bwd" on the luobj refers to the transform direction within the
 * already-inverted pipeline, not the ICC profile direction.
 *
 * Returns 0 on success, nonzero on error.
 */
int argyll_transform_u16(
    const unsigned char *src_icc_data, size_t src_icc_len,
    const unsigned char *dst_icc_data, size_t dst_icc_len,
    int intent,
    const unsigned short *src_pixels,
    unsigned short *dst_pixels,
    size_t npixels
) {
    icmErr e = { 0, { '\0' } };
    icmFile *src_fp = NULL, *dst_fp = NULL;
    icc *src_icc = NULL, *dst_icc = NULL;
    icmLuSpace *src_lu = NULL, *dst_lu = NULL;
    int rv = -1;
    icRenderingIntent ri = map_intent(intent);

    /* Parse source profile */
    src_fp = new_icmFileMem(&e, (void *)src_icc_data, src_icc_len);
    if (src_fp == NULL) goto cleanup;

    src_icc = new_icc(&e);
    if (src_icc == NULL) goto cleanup;

    if (src_icc->read(src_icc, src_fp, 0) != 0) goto cleanup;

    /* Parse destination profile */
    dst_fp = new_icmFileMem(&e, (void *)dst_icc_data, dst_icc_len);
    if (dst_fp == NULL) goto cleanup;

    dst_icc = new_icc(&e);
    if (dst_icc == NULL) goto cleanup;

    if (dst_icc->read(dst_icc, dst_fp, 0) != 0) goto cleanup;

    /* Forward lookup: source device RGB -> PCS XYZ */
    src_lu = (icmLuSpace *)src_icc->get_luobj(src_icc, icmFwd, ri,
                                               icSigXYZData, icmLuOrdNorm);
    if (src_lu == NULL) goto cleanup;

    /* Backward lookup object: will invert dst profile (PCS XYZ -> device RGB) */
    dst_lu = (icmLuSpace *)dst_icc->get_luobj(dst_icc, icmBwd, ri,
                                               icSigXYZData, icmLuOrdNorm);
    if (dst_lu == NULL) goto cleanup;

    /* Transform each pixel: src device -> XYZ -> dst device */
    for (size_t i = 0; i < npixels; i++) {
        double in[3], pcs[3], out[3];
        const unsigned short *sp = &src_pixels[i * 3];
        unsigned short *dp = &dst_pixels[i * 3];

        /* u16 -> double [0, 1] */
        in[0] = sp[0] / 65535.0;
        in[1] = sp[1] / 65535.0;
        in[2] = sp[2] / 65535.0;

        /* Forward on src: device RGB -> XYZ (ignore clip warnings) */
        if (src_lu->lookup_fwd(src_lu, pcs, in) & icmPe_lurv_err)
            goto cleanup;

        /* Forward on backward-obj: XYZ -> device RGB
         * (on an icmBwd luobj, lookup_fwd goes PCS->device) */
        if (dst_lu->lookup_fwd(dst_lu, out, pcs) & icmPe_lurv_err)
            goto cleanup;

        /* double [0, 1] -> u16 with clamping */
        for (int c = 0; c < 3; c++) {
            double v = out[c] * 65535.0 + 0.5;
            if (v < 0.0) v = 0.0;
            if (v > 65535.0) v = 65535.0;
            dp[c] = (unsigned short)v;
        }
    }

    rv = 0;

cleanup:
    if (src_lu) src_lu->del(src_lu);
    if (dst_lu) dst_lu->del(dst_lu);
    if (src_icc) src_icc->del(src_icc);
    if (dst_icc) dst_icc->del(dst_icc);
    if (src_fp) src_fp->del(src_fp);
    if (dst_fp) dst_fp->del(dst_fp);

    return rv;
}
