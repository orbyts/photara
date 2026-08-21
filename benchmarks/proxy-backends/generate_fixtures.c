#include <vips/vips.h>

#include <stdio.h>
#include <stdlib.h>

static VipsImage *make_pattern(int width, int height, double peak) {
    VipsImage *xy = NULL;
    VipsImage *x = NULL;
    VipsImage *y = NULL;
    VipsImage *xf = NULL;
    VipsImage *yf = NULL;
    VipsImage *sum = NULL;
    VipsImage *blue = NULL;
    VipsImage *joined = NULL;
    VipsImage *noise_red = NULL;
    VipsImage *noise_green = NULL;
    VipsImage *noise_blue = NULL;
    VipsImage *noise = NULL;
    VipsImage *textured = NULL;
    VipsImage *memory = NULL;
    VipsImage *bands[3];
    VipsImage *noise_bands[3];

    if (vips_xyz(&xy, width, height, NULL) ||
        vips_extract_band(xy, &x, 0, NULL) ||
        vips_extract_band(xy, &y, 1, NULL) ||
        vips_linear1(x, &xf, peak / (width - 1), 0.0, NULL) ||
        vips_linear1(y, &yf, peak / (height - 1), 0.0, NULL) ||
        vips_add(xf, yf, &sum, NULL) ||
        vips_linear1(sum, &blue, 0.5, 0.0, NULL)) {
        goto error;
    }

    bands[0] = xf;
    bands[1] = yf;
    bands[2] = blue;
    if (vips_bandjoin(bands, &joined, 3, NULL) ||
        vips_gaussnoise(&noise_red, width, height, "mean", 0.0, "sigma", peak / 100.0, "seed",
                        101, NULL) ||
        vips_gaussnoise(&noise_green, width, height, "mean", 0.0, "sigma", peak / 100.0,
                        "seed", 202, NULL) ||
        vips_gaussnoise(&noise_blue, width, height, "mean", 0.0, "sigma", peak / 100.0, "seed",
                        303, NULL)) {
        goto error;
    }
    noise_bands[0] = noise_red;
    noise_bands[1] = noise_green;
    noise_bands[2] = noise_blue;
    if (vips_bandjoin(noise_bands, &noise, 3, NULL) || vips_add(joined, noise, &textured, NULL)) {
        goto error;
    }
    memory = vips_image_copy_memory(textured);
    if (memory == NULL) {
        goto error;
    }

    /* Asymmetric primaries and neutral patches expose orientation and color errors. */
    double red[] = {peak, 0.0, 0.0};
    double green[] = {0.0, peak, 0.0};
    double blue_ink[] = {0.0, 0.0, peak};
    double white[] = {peak, peak, peak};
    if (vips_draw_rect(memory, red, 3, 0, 0, width / 5, height / 5, "fill", TRUE, NULL) ||
        vips_draw_rect(memory, green, 3, width - width / 5, 0, width / 5, height / 5,
                       "fill", TRUE, NULL) ||
        vips_draw_rect(memory, blue_ink, 3, 0, height - height / 5, width / 5, height / 5,
                       "fill", TRUE, NULL) ||
        vips_draw_rect(memory, white, 3, width - width / 5, height - height / 5,
                       width / 5, height / 5, "fill", TRUE, NULL)) {
        goto error;
    }

    g_object_unref(xy);
    g_object_unref(x);
    g_object_unref(y);
    g_object_unref(xf);
    g_object_unref(yf);
    g_object_unref(sum);
    g_object_unref(blue);
    g_object_unref(joined);
    g_object_unref(noise_red);
    g_object_unref(noise_green);
    g_object_unref(noise_blue);
    g_object_unref(noise);
    g_object_unref(textured);
    return memory;

error:
    g_clear_object(&xy);
    g_clear_object(&x);
    g_clear_object(&y);
    g_clear_object(&xf);
    g_clear_object(&yf);
    g_clear_object(&sum);
    g_clear_object(&blue);
    g_clear_object(&joined);
    g_clear_object(&noise_red);
    g_clear_object(&noise_green);
    g_clear_object(&noise_blue);
    g_clear_object(&noise);
    g_clear_object(&textured);
    g_clear_object(&memory);
    return NULL;
}

static int save_sdr(const char *path, const char *profile) {
    VipsImage *pattern = make_pattern(8000, 5333, 1.0);
    VipsImage *scaled = NULL;
    VipsImage *pixels = NULL;
    int result = -1;
    if (pattern == NULL || vips_linear1(pattern, &scaled, 65535.0, 0.0, NULL) ||
        vips_cast(scaled, &pixels, VIPS_FORMAT_USHORT, NULL)) {
        goto done;
    }
    vips_image_set_int(pixels, VIPS_META_ORIENTATION, 1);
    result = vips_tiffsave(pixels, path, "compression", VIPS_FOREIGN_TIFF_COMPRESSION_DEFLATE,
                           "predictor", VIPS_FOREIGN_TIFF_PREDICTOR_HORIZONTAL, "tile", TRUE,
                           "tile-width", 256, "tile-height", 256, "profile", profile, NULL);
done:
    g_clear_object(&pattern);
    g_clear_object(&scaled);
    g_clear_object(&pixels);
    return result;
}

static int save_hdr(const char *path, const char *profile) {
    VipsImage *pattern = make_pattern(8000, 5333, 4.0);
    int result = -1;
    if (pattern == NULL) {
        return -1;
    }
    vips_image_set_int(pattern, VIPS_META_ORIENTATION, 1);
    result = vips_tiffsave(pattern, path, "compression", VIPS_FOREIGN_TIFF_COMPRESSION_DEFLATE,
                           "predictor", VIPS_FOREIGN_TIFF_PREDICTOR_FLOAT, "tile", TRUE,
                           "tile-width", 256, "tile-height", 256, "profile", profile, NULL);
    g_object_unref(pattern);
    return result;
}

int main(int argc, char **argv) {
    if (argc != 5) {
        fprintf(stderr, "usage: %s OUTPUT_DIR DISPLAY_P3_ICC ACESCG_ICC MANIFEST_PATH\n", argv[0]);
        return 2;
    }
    if (VIPS_INIT(argv[0])) {
        vips_error_exit(NULL);
    }

    char sdr_path[4096];
    char hdr_path[4096];
    snprintf(sdr_path, sizeof(sdr_path), "%s/paired-sdr-display-p3-u16.tiff", argv[1]);
    snprintf(hdr_path, sizeof(hdr_path), "%s/paired-hdr-acescg-f32.tiff", argv[1]);

    if (save_sdr(sdr_path, argv[2]) || save_hdr(hdr_path, argv[3])) {
        vips_error_exit("fixture generation failed");
    }

    FILE *manifest = fopen(argv[4], "w");
    if (manifest == NULL) {
        perror("manifest");
        return 1;
    }
    fprintf(manifest,
            "{\n"
            "  \"generator\": \"libvips %s + ImageMagick orientation writer\",\n"
            "  \"fixtures\": [\n"
            "    {\"file\": \"paired-sdr-display-p3-u16.tiff\", \"width\": 8000, "
            "\"height\": 5333, \"sample\": \"u16\", \"icc\": \"Display P3\", "
            "\"orientation\": 1, \"peak\": 1.0},\n"
            "    {\"file\": \"paired-hdr-acescg-f32.tiff\", \"width\": 8000, "
            "\"height\": 5333, \"sample\": \"f32\", \"icc\": \"ACESCG Linear\", "
            "\"orientation\": 1, \"peak\": 4.0},\n"
            "    {\"file\": \"orientation-6-display-p3-u16.tiff\", \"width\": 1600, "
            "\"height\": 1000, \"sample\": \"u16\", \"icc\": \"Display P3\", "
            "\"orientation\": 6, \"peak\": 1.0}\n"
            "  ]\n"
            "}\n",
            vips_version_string());
    fclose(manifest);
    vips_shutdown();
    return 0;
}
