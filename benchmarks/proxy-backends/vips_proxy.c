#include <vips/vips.h>

#include <stdio.h>
#include <string.h>

static int load_oriented(const char *path, VipsImage **out) {
    VipsImage *loaded = NULL;
    if (vips_tiffload(path, &loaded, "access", VIPS_ACCESS_SEQUENTIAL, NULL) ||
        vips_autorot(loaded, out, NULL)) {
        g_clear_object(&loaded);
        return -1;
    }
    g_object_unref(loaded);
    return 0;
}

static int resize_long_edge(VipsImage *input, VipsImage **out, int long_edge) {
    const int source_long_edge =
        vips_image_get_width(input) > vips_image_get_height(input)
            ? vips_image_get_width(input)
            : vips_image_get_height(input);
    const double scale = source_long_edge > long_edge
                             ? (double)long_edge / (double)source_long_edge
                             : 1.0;
    return vips_resize(input, out, scale, "kernel", VIPS_KERNEL_LANCZOS3, NULL);
}

static int thumbnail_sdr(const char *input_path, const char *output_path,
                         const char *output_profile, int long_edge) {
    VipsImage *oriented = NULL;
    VipsImage *linear = NULL;
    VipsImage *resized = NULL;
    VipsImage *encoded = NULL;
    int result = -1;

    if (load_oriented(input_path, &oriented) ||
        vips_icc_import(oriented, &linear, "embedded", TRUE, "pcs", VIPS_PCS_XYZ, "intent",
                        VIPS_INTENT_RELATIVE, "black-point-compensation", TRUE, NULL) ||
        resize_long_edge(linear, &resized, long_edge) ||
        vips_icc_export(resized, &encoded, "output-profile", output_profile, "pcs", VIPS_PCS_XYZ,
                        "intent", VIPS_INTENT_RELATIVE, "black-point-compensation", TRUE, "depth",
                        8, NULL)) {
        goto done;
    }
    vips_image_set_int(encoded, VIPS_META_ORIENTATION, 1);
    result = vips_pngsave(encoded, output_path, "bitdepth", 8, "profile", output_profile, NULL);

done:
    g_clear_object(&oriented);
    g_clear_object(&linear);
    g_clear_object(&resized);
    g_clear_object(&encoded);
    return result;
}

static int authoring_hdr(const char *input_path, const char *output_path,
                         const char *source_profile, int long_edge) {
    VipsImage *oriented = NULL;
    VipsImage *resized = NULL;
    int result = -1;

    if (load_oriented(input_path, &oriented) ||
        resize_long_edge(oriented, &resized, long_edge)) {
        goto done;
    }
    vips_image_set_int(resized, VIPS_META_ORIENTATION, 1);
    result = vips_tiffsave(
        resized, output_path, "compression", VIPS_FOREIGN_TIFF_COMPRESSION_DEFLATE, "predictor",
        VIPS_FOREIGN_TIFF_PREDICTOR_FLOAT, "tile", TRUE, "tile-width", 256, "tile-height", 256,
        "profile", source_profile, NULL);

done:
    g_clear_object(&oriented);
    g_clear_object(&resized);
    return result;
}

int main(int argc, char **argv) {
    if (argc != 6) {
        fprintf(stderr, "usage: %s MODE INPUT ICC LONG_EDGE OUTPUT\n", argv[0]);
        return 2;
    }
    if (VIPS_INIT(argv[0])) {
        vips_error_exit(NULL);
    }

    const int long_edge = atoi(argv[4]);
    if (long_edge <= 0) {
        fprintf(stderr, "LONG_EDGE must be positive\n");
        return 2;
    }

    int result;
    if (strcmp(argv[1], "thumbnail-sdr") == 0) {
        result = thumbnail_sdr(argv[2], argv[5], argv[3], long_edge);
    } else if (strcmp(argv[1], "authoring-hdr") == 0) {
        result = authoring_hdr(argv[2], argv[5], argv[3], long_edge);
    } else {
        fprintf(stderr, "unknown mode %s\n", argv[1]);
        return 2;
    }

    if (result != 0) {
        vips_error_exit("proxy failed");
    }
    vips_shutdown();
    return 0;
}
