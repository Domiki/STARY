use rayon::prelude::*;
use stary_mask::Mask;
use stary_register::{AlignError, AlignParams, Alignment, Star, Transform, align};

use super::prepare::Prepared;
use crate::error::PipelineError;

/// The sky of the reference frame: only the stars in it are aligned on.
pub struct SkyView<'a> {
    pub mask: &'a Mask,
    pub width: usize,
    pub height: usize,
}

impl SkyView<'_> {
    fn stars_in_sky(&self, stars: &[Star]) -> Vec<Star> {
        stars
            .iter()
            .filter(|star| self.mask.is_sky_at(star.x, star.y, self.width, self.height))
            .copied()
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reference {
    /// The frame at this position in the list of lights (the last one when it is past the end).
    Index(usize),
    /// The frame with the most stars (ties go to the sharper one).
    Best,
}

pub struct Registration {
    pub reference: usize,
    /// How each frame maps onto the reference; `Err` for frames that could not be aligned.
    pub outcomes: Vec<Result<Alignment, AlignError>>,
}

fn median_fwhm(prepared: &Prepared) -> f32 {
    let mut sizes: Vec<f32> = prepared.stars.iter().map(|star| star.fwhm).collect();
    if sizes.is_empty() {
        return f32::MAX;
    }
    sizes.sort_by(f32::total_cmp);
    sizes[sizes.len() / 2]
}

pub fn choose_reference(frames: &[Prepared], choice: Reference) -> usize {
    match choice {
        Reference::Index(index) => index.min(frames.len().saturating_sub(1)),
        Reference::Best => (0..frames.len())
            .max_by(|&a, &b| {
                frames[a]
                    .stars
                    .len()
                    .cmp(&frames[b].stars.len())
                    .then(median_fwhm(&frames[b]).total_cmp(&median_fwhm(&frames[a])))
            })
            .unwrap_or(0),
    }
}

pub fn register(
    frames: &[Prepared],
    choice: Reference,
    params: &AlignParams,
    sky: Option<&SkyView>,
) -> Result<Registration, PipelineError> {
    let reference = choose_reference(frames, choice);
    let reference_stars = match sky {
        Some(sky) => sky.stars_in_sky(&frames[reference].stars),
        None => frames[reference].stars.clone(),
    };
    if sky.is_some() && reference_stars.len() < params.min_inliers {
        return Err(PipelineError::NotEnoughSkyStars);
    }
    let outcomes = (0..frames.len())
        .into_par_iter()
        .map(|index| {
            if index == reference {
                return Ok(Alignment {
                    transform: Transform::IDENTITY,
                    inliers: reference_stars.len(),
                    rms: 0.0,
                });
            }
            align(&reference_stars, &frames[index].stars, params)
        })
        .collect();
    Ok(Registration {
        reference,
        outcomes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use stary_register::Star;
    use stary_stack::SkyLevel;

    fn prepared(stars: usize, fwhm: f32) -> Prepared {
        Prepared {
            stars: (0..stars)
                .map(|i| Star {
                    x: i as f64,
                    y: 0.0,
                    flux: 1.0,
                    peak: 1.0,
                    fwhm,
                    roundness: 1.0,
                })
                .collect(),
            sky: [SkyLevel {
                location: 0.0,
                scale: 1.0,
            }; 3],
            planes: None,
        }
    }

    #[test]
    fn a_chosen_frame_is_the_reference() {
        let frames = [prepared(10, 3.0), prepared(50, 3.0), prepared(20, 3.0)];
        assert_eq!(choose_reference(&frames, Reference::Index(0)), 0);
        assert_eq!(choose_reference(&frames, Reference::Index(2)), 2);
    }

    #[test]
    fn a_position_past_the_end_falls_back_to_the_last_frame() {
        let frames = [prepared(10, 3.0), prepared(50, 3.0)];
        assert_eq!(choose_reference(&frames, Reference::Index(9)), 1);
    }

    #[test]
    fn the_best_reference_has_the_most_stars_then_the_sharpest() {
        let frames = [prepared(10, 3.0), prepared(50, 4.0), prepared(50, 3.0)];
        assert_eq!(choose_reference(&frames, Reference::Best), 2);
    }
}
