use std::{path::PathBuf, time::Instant};

use anyhow::{Context, anyhow, bail};
use image::{GrayImage, RgbImage};
use stary_mask::shaped;
use stary_sky::{Contour, Paint, Picture, Sky, Stroke};

#[derive(clap::Args)]
pub struct SkyArgs {
    /// A photograph or a raw file; the sky is looked for in its preview
    image: PathBuf,
    /// The mask as a gray PNG: white is sky, black is ground
    #[arg(long)]
    out: PathBuf,
    /// A picture of the border found, over the photograph
    #[arg(long)]
    overlay: Option<PathBuf>,
    #[arg(long, default_value_t = 1600)]
    max_edge: u32,
    /// Pixels the border is moved: positive grows the sky
    #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
    offset: f32,
    /// Pixels over which the mask falls from sky to ground
    #[arg(long, default_value_t = 0.0)]
    feather: f32,
    /// A dab of sky, as x,y,radius in pixels of the preview (may be repeated)
    #[arg(long)]
    sky: Vec<String>,
    /// A dab of ground, as x,y,radius
    #[arg(long)]
    ground: Vec<String>,
    /// A dab that finds the border near it again at a finer scale, as x,y,radius
    #[arg(long)]
    refine: Vec<String>,
}

fn dabs(values: &[String], paint: Paint) -> anyhow::Result<Vec<Stroke>> {
    values
        .iter()
        .map(|value| {
            let numbers: Vec<f32> = value
                .split(',')
                .map(|part| part.trim().parse::<f32>())
                .collect::<Result<_, _>>()
                .with_context(|| format!("not x,y,radius: {value}"))?;
            let [x, y, radius] = numbers[..] else {
                bail!("not x,y,radius: {value}");
            };
            Ok(Stroke {
                paint,
                radius,
                points: vec![[x, y]],
            })
        })
        .collect()
}

fn draw_line(image: &mut RgbImage, from: [f32; 2], to: [f32; 2]) {
    let steps = (to[0] - from[0])
        .abs()
        .max((to[1] - from[1]).abs())
        .ceil()
        .max(1.0) as u32;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let (x, y) = (
            from[0] + t * (to[0] - from[0]),
            from[1] + t * (to[1] - from[1]),
        );
        if x >= 0.0 && y >= 0.0 && (x as u32) < image.width() && (y as u32) < image.height() {
            image.put_pixel(x as u32, y as u32, image::Rgb([255, 230, 0]));
        }
    }
}

fn overlay(rgb: &[u8], width: u32, height: u32, sky: &[bool], contours: &[Contour]) -> RgbImage {
    let mut image = RgbImage::from_raw(width, height, rgb.to_vec()).expect("a matching buffer");
    for (pixel, &inside) in image.pixels_mut().zip(sky) {
        if !inside {
            pixel.0 = pixel.0.map(|value| (f32::from(value) * 0.45) as u8);
            pixel.0[0] = pixel.0[0].saturating_add(70);
        }
    }
    for contour in contours {
        for pair in contour.windows(2) {
            draw_line(&mut image, pair[0], pair[1]);
        }
    }
    image
}

pub fn run(args: &SkyArgs) -> anyhow::Result<()> {
    let preview = stary_io::read_preview(&args.image, args.max_edge)?;
    let (width, height) = (preview.width as usize, preview.height as usize);
    let picture = Picture {
        width,
        height,
        rgb: &preview.rgb8,
    };
    let mut strokes = dabs(&args.sky, Paint::Sky)?;
    strokes.extend(dabs(&args.ground, Paint::Ground)?);
    strokes.extend(dabs(&args.refine, Paint::Refine)?);

    let started = Instant::now();
    let sky = Sky::analyze(&picture).map_err(|error| anyhow!("{error}"))?;
    let analysed = started.elapsed().as_secs_f64();
    let detection = sky.detect(&strokes);
    println!(
        "{width}x{height}: analysed in {analysed:.2} s, found in {:.2} s, {:.1}% sky",
        started.elapsed().as_secs_f64() - analysed,
        detection.sky_fraction() * 100.0
    );

    let mask = shaped(width, height, &detection.sky, args.offset, args.feather)
        .map_err(|error| anyhow!("{error}"))?;
    let bytes: Vec<u8> = mask
        .values()
        .iter()
        .map(|value| (value * 255.0).round() as u8)
        .collect();
    GrayImage::from_raw(preview.width, preview.height, bytes)
        .ok_or_else(|| anyhow!("mask buffer has unexpected size"))?
        .save(&args.out)
        .with_context(|| format!("cannot write {}", args.out.display()))?;
    if let Some(path) = &args.overlay {
        let contours = detection.contours(0.8);
        overlay(
            &preview.rgb8,
            preview.width,
            preview.height,
            &detection.sky,
            &contours,
        )
        .save(path)
        .with_context(|| format!("cannot write {}", path.display()))?;
    }
    Ok(())
}
