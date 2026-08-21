use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write as _},
    path::Path,
};

use image::{
    DynamicImage, ExtendedColorType, ImageDecoder as _, ImageEncoder as _, ImageReader,
    codecs::{png::PngEncoder, tiff::TiffEncoder},
    imageops::FilterType,
};
use photara_core::{ProxyProfile, ProxySizing};

struct DecodedImage {
    image: DynamicImage,
    icc: Option<Vec<u8>>,
}

fn profile<'a>(profiles: &'a [ProxyProfile], suffix: &str) -> Result<&'a ProxyProfile, String> {
    profiles
        .iter()
        .find(|profile| profile.id.as_str().ends_with(suffix))
        .ok_or_else(|| format!("missing {suffix} contract"))
}

fn long_edge(profile: &ProxyProfile) -> u32 {
    match profile.sizing {
        ProxySizing::LongEdge { pixels } => pixels.get(),
        ProxySizing::FitWithin {
            max_width,
            max_height,
        } => max_width.get().max(max_height.get()),
    }
}

fn decode(path: &Path) -> Result<DecodedImage, Box<dyn Error>> {
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation()?;
    let icc = decoder.icc_profile()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok(DecodedImage { image, icc })
}

fn thumbnail_sdr(
    input: &Path,
    output: &Path,
    profile: &ProxyProfile,
) -> Result<(), Box<dyn Error>> {
    let decoded = decode(input)?;
    let size = long_edge(profile);
    let resized = decoded
        .image
        .resize(size, size, FilterType::Lanczos3)
        .to_rgb8();
    let file = BufWriter::new(File::create(output)?);
    let mut encoder = PngEncoder::new(file);
    // Deliberately retain the source profile: image-rs exposes ICC bytes but
    // does not perform the contract's Display-P3-to-sRGB color transform.
    if let Some(icc) = decoded.icc {
        encoder.set_icc_profile(icc)?;
    }
    encoder.write_image(
        resized.as_raw(),
        resized.width(),
        resized.height(),
        ExtendedColorType::Rgb8,
    )?;
    Ok(())
}

fn authoring_hdr(
    input: &Path,
    output: &Path,
    profile: &ProxyProfile,
) -> Result<(), Box<dyn Error>> {
    let decoded = decode(input)?;
    let size = long_edge(profile);
    let resized = decoded
        .image
        .resize(size, size, FilterType::Lanczos3)
        .to_rgb32f();
    let mut bytes = Vec::with_capacity(resized.as_raw().len() * 4);
    for sample in resized.as_raw() {
        bytes.write_all(&sample.to_ne_bytes())?;
    }
    let file = BufWriter::new(File::create(output)?);
    let mut encoder = TiffEncoder::new(file);
    if let Some(icc) = decoded.icc {
        encoder.set_icc_profile(icc)?;
    }
    encoder.write_image(
        &bytes,
        resized.width(),
        resized.height(),
        ExtendedColorType::Rgb32F,
    )?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = env::args_os().collect();
    if arguments.len() != 6 {
        return Err("usage: rust-image-proxy MODE INPUT ICC CONTRACTS OUTPUT".into());
    }
    let mode = arguments[1].to_string_lossy();
    let input = Path::new(&arguments[2]);
    let output = Path::new(&arguments[5]);
    let contracts: Vec<ProxyProfile> = serde_json::from_slice(&fs::read(&arguments[4])?)?;

    match mode.as_ref() {
        "thumbnail-sdr" => thumbnail_sdr(input, output, profile(&contracts, "thumbnail-sdr")?)?,
        "authoring-hdr" => authoring_hdr(input, output, profile(&contracts, "authoring-hdr")?)?,
        _ => return Err(format!("unknown mode {mode}").into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_contracts_track_core_schema() {
        let profiles: Vec<ProxyProfile> =
            serde_json::from_str(include_str!("../../contracts.json")).unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(long_edge(&profiles[0]), 512);
        assert_eq!(long_edge(&profiles[1]), 2048);
    }
}
