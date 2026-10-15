use super::prepare::Prepared;

const SAMPLE_STARS: usize = 50;
const MIN_STARS_FOR_SHAPE: usize = 5;
/// With fewer stars than this in a typical frame the star counts say little (a sparse field).
const MIN_TYPICAL_STARS: f32 = 20.0;
const FEW_STARS: f32 = 0.4;
const BLURRY: f32 = 1.5;
const TRAILED: f32 = 0.75;
const NOISY: f32 = 1.5;
/// The sky counts as brighter when it exceeds the typical one by this share of it and ...
const BRIGHT_SKY: f32 = 0.25;
/// ... by this many robust standard deviations of the sky levels of all frames.
const BRIGHT_SKY_SIGMAS: f32 = 6.0;
const MAD_TO_SIGMA: f32 = 1.4826;

/// What a frame looks like, judged from its stars and its sky.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameQuality {
    pub stars: usize,
    /// Width of the brightest stars; larger means blurrier.
    pub fwhm: f32,
    /// 1 for round stars, less for trailed ones.
    pub roundness: f32,
    /// The sky level and its noise, averaged over the colors.
    pub background: f32,
    pub noise: f32,
}

fn median(values: &mut [f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f32::total_cmp);
    values[values.len() / 2]
}

/// Why a frame is left out of a stack for its quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poor {
    /// Clouds, haze, moonlight or a light: the sky is far brighter than in the other frames.
    BrightSky,
    Noisy,
    FewStars,
    Blurry,
    Trailed,
}

impl std::fmt::Display for Poor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::BrightSky => "the sky is much brighter than in the other frames",
            Self::Noisy => "the frame is much noisier than the others",
            Self::FewStars => "far fewer stars than in the other frames",
            Self::Blurry => "the stars are much blurrier than in the other frames",
            Self::Trailed => "the stars are trailed",
        })
    }
}

fn spread(values: &[f32], center: f32) -> f32 {
    let mut deviations: Vec<f32> = values.iter().map(|value| (value - center).abs()).collect();
    MAD_TO_SIGMA * median(&mut deviations)
}

fn judge(
    frame: &FrameQuality,
    typical: &FrameQuality,
    sky_spread: f32,
    noise_spread: f32,
) -> Option<Poor> {
    let brighter = frame.background - typical.background;
    if brighter > BRIGHT_SKY * typical.background.abs() && brighter > BRIGHT_SKY_SIGMAS * sky_spread
    {
        return Some(Poor::BrightSky);
    }
    let noisier = frame.noise - typical.noise;
    if frame.noise > NOISY * typical.noise && noisier > BRIGHT_SKY_SIGMAS * noise_spread {
        return Some(Poor::Noisy);
    }
    if typical.stars as f32 >= MIN_TYPICAL_STARS
        && (frame.stars as f32) < FEW_STARS * typical.stars as f32
    {
        return Some(Poor::FewStars);
    }
    if frame.fwhm > 0.0 && typical.fwhm > 0.0 {
        if frame.fwhm > BLURRY * typical.fwhm {
            return Some(Poor::Blurry);
        }
        if frame.roundness < TRAILED * typical.roundness {
            return Some(Poor::Trailed);
        }
    }
    None
}

/// Finds the frames that are much worse than the rest: judged against the typical frame (the
/// median of each measure), so a whole night of mediocre frames is not thrown away. `keep` is a
/// frame that stays whatever its quality (the reference frame the user chose).
pub fn assess(frames: &[FrameQuality], keep: usize) -> Vec<Option<Poor>> {
    if frames.len() < 3 {
        return vec![None; frames.len()];
    }
    let column = |pick: fn(&FrameQuality) -> f32| -> Vec<f32> { frames.iter().map(pick).collect() };
    let center = |values: &Vec<f32>| median(&mut values.clone());
    let (backgrounds, noises) = (column(|f| f.background), column(|f| f.noise));
    let shaped = |pick: fn(&FrameQuality) -> f32| -> f32 {
        let mut values: Vec<f32> = frames.iter().filter(|f| f.fwhm > 0.0).map(pick).collect();
        median(&mut values)
    };
    let typical = FrameQuality {
        stars: center(&column(|f| f.stars as f32)).round() as usize,
        fwhm: shaped(|f| f.fwhm),
        roundness: shaped(|f| f.roundness),
        background: center(&backgrounds),
        noise: center(&noises),
    };
    let sky_spread = spread(&backgrounds, typical.background);
    let noise_spread = spread(&noises, typical.noise);
    frames
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            judge(frame, &typical, sky_spread, noise_spread).filter(|_| index != keep)
        })
        .collect()
}

pub fn measure(frame: &Prepared) -> FrameQuality {
    let brightest = &frame.stars[..frame.stars.len().min(SAMPLE_STARS)];
    let shape = brightest.len() >= MIN_STARS_FOR_SHAPE;
    let mut widths: Vec<f32> = brightest.iter().map(|star| star.fwhm).collect();
    let mut roundness: Vec<f32> = brightest.iter().map(|star| star.roundness).collect();
    let colors = frame.sky.len() as f32;
    FrameQuality {
        stars: frame.stars.len(),
        fwhm: if shape { median(&mut widths) } else { 0.0 },
        roundness: if shape { median(&mut roundness) } else { 0.0 },
        background: frame.sky.iter().map(|sky| sky.location).sum::<f32>() / colors,
        noise: frame.sky.iter().map(|sky| sky.scale).sum::<f32>() / colors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stary_register::Star;
    use stary_stack::SkyLevel;

    fn star(fwhm: f32, roundness: f32) -> Star {
        Star {
            x: 0.0,
            y: 0.0,
            flux: 1.0,
            peak: 1.0,
            fwhm,
            roundness,
        }
    }

    fn frame(stars: Vec<Star>) -> Prepared {
        Prepared {
            stars,
            sky: [
                SkyLevel {
                    location: 100.0,
                    scale: 3.0,
                },
                SkyLevel {
                    location: 200.0,
                    scale: 6.0,
                },
                SkyLevel {
                    location: 300.0,
                    scale: 9.0,
                },
            ],
            planes: None,
        }
    }

    #[test]
    fn the_shape_is_the_median_of_the_brightest_stars() {
        let stars = (0..10).map(|i| star(4.0 + i as f32, 0.9)).collect();
        let quality = measure(&frame(stars));
        assert_eq!(quality.stars, 10);
        assert_eq!(quality.fwhm, 9.0);
        assert_eq!(quality.roundness, 0.9);
    }

    #[test]
    fn the_sky_is_averaged_over_the_colors() {
        let quality = measure(&frame(Vec::new()));
        assert_eq!((quality.background, quality.noise), (200.0, 6.0));
    }

    fn good() -> FrameQuality {
        FrameQuality {
            stars: 400,
            fwhm: 20.0,
            roundness: 0.85,
            background: 1000.0,
            noise: 100.0,
        }
    }

    fn night(count: usize) -> Vec<FrameQuality> {
        (0..count)
            .map(|index| FrameQuality {
                fwhm: 20.0 + (index % 3) as f32,
                background: 1000.0 + (index % 5) as f32 * 8.0,
                noise: 100.0 + (index % 4) as f32,
                ..good()
            })
            .collect()
    }

    #[test]
    fn an_even_night_loses_no_frame() {
        assert!(assess(&night(20), 0).iter().all(Option::is_none));
    }

    #[test]
    fn each_kind_of_failure_is_named() {
        let mut frames = night(20);
        frames[1].background = 4000.0;
        frames[2].noise = 400.0;
        frames[3].stars = 50;
        frames[4].fwhm = 40.0;
        frames[5].roundness = 0.5;
        let verdict = assess(&frames, 0);
        assert_eq!(verdict[1], Some(Poor::BrightSky));
        assert_eq!(verdict[2], Some(Poor::Noisy));
        assert_eq!(verdict[3], Some(Poor::FewStars));
        assert_eq!(verdict[4], Some(Poor::Blurry));
        assert_eq!(verdict[5], Some(Poor::Trailed));
        assert_eq!(verdict.iter().flatten().count(), 5);
    }

    #[test]
    fn the_frame_that_must_stay_is_never_judged_poor() {
        let mut frames = night(10);
        frames[3].background = 9000.0;
        assert_eq!(assess(&frames, 3)[3], None);
        assert_eq!(assess(&frames, 0)[3], Some(Poor::BrightSky));
    }

    #[test]
    fn a_slowly_brightening_sky_is_not_poor() {
        let frames: Vec<FrameQuality> = (0..20)
            .map(|index| FrameQuality {
                background: 1000.0 + 15.0 * index as f32,
                ..good()
            })
            .collect();
        assert!(assess(&frames, 0).iter().all(Option::is_none));
    }

    #[test]
    fn too_few_frames_to_judge() {
        let mut frames = night(2);
        frames[1].background = 9000.0;
        assert!(assess(&frames, 0).iter().all(Option::is_none));
    }

    #[test]
    fn a_sparse_field_is_not_judged_by_its_star_counts() {
        let mut frames: Vec<FrameQuality> = (0..10)
            .map(|_| FrameQuality {
                stars: 12,
                ..good()
            })
            .collect();
        frames[2].stars = 2;
        assert!(assess(&frames, 0).iter().all(Option::is_none));
    }

    #[test]
    fn a_frame_with_few_stars_has_no_shape() {
        let quality = measure(&frame(vec![star(5.0, 0.9); 4]));
        assert_eq!((quality.fwhm, quality.roundness), (0.0, 0.0));
        assert_eq!(quality.stars, 4);
    }
}
