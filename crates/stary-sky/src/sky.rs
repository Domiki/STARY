use crate::{
    cleanup::without_specks,
    contour::{Contour, contours},
    error::SkyError,
    mixture::Mixture,
    plane::Plane,
    solver::{Feature, Lattice},
    strokes::{Paint, Stroke, raster},
};

/// A picture to find the sky in: 8-bit RGB, row by row, as it is shown on the screen.
#[derive(Debug, Clone, Copy)]
pub struct Picture<'a> {
    pub width: usize,
    pub height: usize,
    pub rgb: &'a [u8],
}

const MIN_EDGE: usize = 32;
/// The size the tuning below is made for: the long edge of a preview.
const REFERENCE_EDGE: f32 = 1600.0;
const OPENING_RADIUS: usize = 2;
const COARSE_BLUR: f32 = 2.5;
const COLOR_BLUR: f32 = 3.0;
const FINE_BLUR: f32 = 1.2;
const FINE_COLOR_BLUR: f32 = 1.0;
const LIGHT_FLOOR: f32 = 0.01;
const COARSE_SHARPNESS: f32 = 40.0;
const FINE_SHARPNESS: f32 = 60.0;
const DATA_STRENGTH: f32 = 0.05;
const SOLVE_TOLERANCE: f32 = 1e-3;
const COARSE_ROUNDS: usize = 400;
const FINE_ROUNDS: usize = 300;
/// Two curves closer than this (in their own widths) are one kind of sky, not sky and ground.
const MIN_SEPARATION: f32 = 2.5;
const MIN_SHARE: f32 = 0.02;
const SPECK_SHARE: f32 = 0.0003;
const MIN_SPECK: usize = 64;
const REFINE_MARGIN: f32 = 2.0;
/// How far from the border found so far it is looked for again, in pixels of a 1600 pixel picture.
const REFINE_BAND: usize = 7;

/// How the light of a picture looks at one scale: its logarithm (stars taken away) and its colours.
struct Level {
    width: usize,
    height: usize,
    light: Vec<f32>,
    color: [Plane; 3],
}

impl Level {
    fn features(&self, window: Window) -> Vec<Feature> {
        (window.top..window.bottom)
            .flat_map(|y| (window.left..window.right).map(move |x| (x, y)))
            .map(|(x, y)| {
                let index = y * self.width + x;
                [
                    self.light[index],
                    self.color[0].data[index],
                    self.color[1].data[index],
                    self.color[2].data[index],
                ]
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
struct Window {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

impl Window {
    fn whole(width: usize, height: usize) -> Self {
        Self {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        }
    }

    fn width(&self) -> usize {
        self.right - self.left
    }

    fn height(&self) -> usize {
        self.bottom - self.top
    }

    fn indices(self, row: usize) -> impl Iterator<Item = usize> {
        let full = self;
        (full.top..full.bottom).flat_map(move |y| (full.left..full.right).map(move |x| y * row + x))
    }
}

/// Where the sky is in a picture.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    pub width: usize,
    pub height: usize,
    pub sky: Vec<bool>,
}

impl Detection {
    pub fn sky_fraction(&self) -> f32 {
        self.sky.iter().filter(|&&sky| sky).count() as f32 / self.sky.len() as f32
    }

    /// The borders between sky and ground, as lines on the corners of pixels.
    pub fn contours(&self, tolerance: f32) -> Vec<Contour> {
        contours(&self.sky, self.width, self.height, tolerance)
    }
}

/// What is known about a picture that does not change with the strokes painted on it, so that
/// every stroke only costs the part of the work that depends on it.
pub struct Sky {
    width: usize,
    height: usize,
    coarse: Level,
    fine: Level,
    lattice: Lattice,
    mixture: Mixture,
    sky_curve: usize,
    all_sky: bool,
}

fn luminance(picture: &Picture) -> Plane {
    let data = picture
        .rgb
        .as_chunks::<3>()
        .0
        .iter()
        .map(|pixel| {
            (0.2126 * f32::from(pixel[0])
                + 0.7152 * f32::from(pixel[1])
                + 0.0722 * f32::from(pixel[2]))
                / 255.0
        })
        .collect();
    Plane::new(picture.width, picture.height, data)
}

fn channel(picture: &Picture, index: usize) -> Plane {
    let data = picture
        .rgb
        .as_chunks::<3>()
        .0
        .iter()
        .map(|pixel| f32::from(pixel[index]) / 255.0)
        .collect();
    Plane::new(picture.width, picture.height, data)
}

fn logarithm(plane: &Plane) -> Vec<f32> {
    plane.map(|value| (value + LIGHT_FLOOR).ln()).data
}

fn build_levels(picture: &Picture, scale: f32) -> (Level, Level) {
    let light = luminance(picture);
    let colors: [Plane; 3] = std::array::from_fn(|index| channel(picture, index));
    let coarse_light = light
        .opened(OPENING_RADIUS)
        .gaussian(COARSE_BLUR * scale)
        .halved();
    let coarse = Level {
        width: coarse_light.width,
        height: coarse_light.height,
        light: logarithm(&coarse_light),
        color: std::array::from_fn(|index| colors[index].gaussian(COLOR_BLUR * scale).halved()),
    };
    let fine = Level {
        width: picture.width,
        height: picture.height,
        light: logarithm(&light.opened(1).gaussian(FINE_BLUR * scale)),
        color: std::array::from_fn(|index| colors[index].gaussian(FINE_COLOR_BLUR * scale)),
    };
    (coarse, fine)
}

/// The curve whose pixels lie higher in the picture is the sky.
fn sky_curve_of(mixture: &Mixture, level: &Level) -> usize {
    let mut weighted_rows = [0.0f64; 2];
    let mut masses = [0.0f64; 2];
    for (index, &value) in level.light.iter().enumerate() {
        let row = (index / level.width) as f64 / level.height as f64;
        for curve in 0..2 {
            let share = f64::from(mixture.posterior(curve, value));
            weighted_rows[curve] += share * row;
            masses[curve] += share;
        }
    }
    let mean = |curve: usize| weighted_rows[curve] / masses[curve].max(1e-9);
    usize::from(mean(1) < mean(0))
}

/// Which pixels of a window lie within `reach` steps of a pixel of the other kind.
fn near_border(region: &[bool], width: usize, window: Window, reach: usize) -> Vec<bool> {
    let (across, down) = (window.width(), window.height());
    let local = |x: usize, y: usize| region[(window.top + y) * width + window.left + x];
    let mut steps = vec![usize::MAX; across * down];
    let mut frontier = Vec::new();
    for y in 0..down {
        for x in 0..across {
            let here = local(x, y);
            let differs = (x > 0 && local(x - 1, y) != here)
                || (x + 1 < across && local(x + 1, y) != here)
                || (y > 0 && local(x, y - 1) != here)
                || (y + 1 < down && local(x, y + 1) != here);
            if differs {
                steps[y * across + x] = 0;
                frontier.push((x, y));
            }
        }
    }
    for step in 1..=reach {
        let mut next = Vec::new();
        for (x, y) in frontier {
            let around = [
                (x > 0).then(|| (x - 1, y)),
                (x + 1 < across).then(|| (x + 1, y)),
                (y > 0).then(|| (x, y - 1)),
                (y + 1 < down).then(|| (x, y + 1)),
            ];
            for (nx, ny) in around.into_iter().flatten() {
                if steps[ny * across + nx] == usize::MAX {
                    steps[ny * across + nx] = step;
                    next.push((nx, ny));
                }
            }
        }
        frontier = next;
    }
    steps.into_iter().map(|step| step != usize::MAX).collect()
}

impl Sky {
    pub fn analyze(picture: &Picture) -> Result<Self, SkyError> {
        let (width, height) = (picture.width, picture.height);
        if width == 0 || height == 0 || picture.rgb.len() != width * height * 3 {
            return Err(SkyError::BadPicture);
        }
        if width.min(height) < MIN_EDGE {
            return Err(SkyError::TooSmall);
        }
        let scale = width.max(height) as f32 / REFERENCE_EDGE;
        let (coarse, fine) = build_levels(picture, scale);
        let mixture = Mixture::fit(&coarse.light);
        let sky_curve = sky_curve_of(&mixture, &coarse);
        let features = coarse.features(Window::whole(coarse.width, coarse.height));
        let lattice =
            Lattice::from_features(coarse.width, coarse.height, &features, COARSE_SHARPNESS);
        let all_sky = mixture.separation() < MIN_SEPARATION
            || mixture.weights.iter().any(|&weight| weight < MIN_SHARE);
        Ok(Self {
            width,
            height,
            coarse,
            fine,
            lattice,
            mixture,
            sky_curve,
            all_sky,
        })
    }

    fn prior(&self, level: &Level, window: Window) -> Vec<f32> {
        window
            .indices(level.width)
            .map(|index| self.mixture.posterior(self.sky_curve, level.light[index]))
            .collect()
    }

    fn coarse_sky(&self, strokes: &[Stroke]) -> Plane {
        let (width, height) = (self.coarse.width, self.coarse.height);
        if self.all_sky {
            return Plane::filled(width, height, 1.0);
        }
        let prior = self.prior(&self.coarse, Window::whole(width, height));
        let scale = self.width as f32 / width as f32;
        let seeds: Vec<Option<f32>> = raster(strokes, width, height, scale, |paint| {
            paint != Paint::Refine
        })
        .into_iter()
        .map(|paint| match paint {
            Some(Paint::Sky) => Some(1.0),
            Some(_) => Some(0.0),
            None => None,
        })
        .collect();
        let smooth = self.lattice.smooth(
            &prior,
            DATA_STRENGTH,
            &seeds,
            &prior,
            SOLVE_TOLERANCE,
            COARSE_ROUNDS,
        );
        Plane::new(width, height, smooth)
    }

    fn refine(&self, region: &mut [bool], strokes: &[Stroke], forced: &[Option<Paint>]) {
        for stroke in strokes
            .iter()
            .filter(|stroke| stroke.paint == Paint::Refine)
        {
            let reach = stroke.radius * REFINE_MARGIN;
            let widened = Stroke {
                radius: reach,
                ..stroke.clone()
            };
            let Some((left, top, right, bottom)) = widened.bounds(self.width, self.height) else {
                continue;
            };
            let window = Window {
                left,
                top,
                right,
                bottom,
            };
            let near = raster(
                std::slice::from_ref(stroke),
                self.width,
                self.height,
                1.0,
                |_| true,
            );
            let features = self.fine.features(window);
            let lattice =
                Lattice::from_features(window.width(), window.height(), &features, FINE_SHARPNESS);
            let prior = self.prior(&self.fine, window);
            let reach = (REFINE_BAND as f32 * self.width.max(self.height) as f32 / REFERENCE_EDGE)
                .ceil() as usize;
            let band = near_border(region, self.width, window, reach);
            let seeds: Vec<Option<f32>> = window
                .indices(self.width)
                .zip(&band)
                .map(|(index, &in_band)| match (near[index], forced[index]) {
                    (_, Some(Paint::Sky)) => Some(1.0),
                    (_, Some(_)) => Some(0.0),
                    (Some(_), None) if in_band => None,
                    _ => Some(f32::from(u8::from(region[index]))),
                })
                .collect();
            let start: Vec<f32> = window
                .indices(self.width)
                .map(|index| f32::from(u8::from(region[index])))
                .collect();
            let smooth = lattice.smooth(
                &prior,
                DATA_STRENGTH,
                &seeds,
                &start,
                SOLVE_TOLERANCE,
                FINE_ROUNDS,
            );
            for (index, value) in window.indices(self.width).zip(smooth) {
                if near[index].is_some() && forced[index].is_none() {
                    region[index] = value >= 0.5;
                }
            }
        }
    }

    /// Finds the sky. Strokes of sky and ground are taken as true; strokes to refine make the
    /// border near them be found again at the scale of single pixels.
    pub fn detect(&self, strokes: &[Stroke]) -> Detection {
        let coarse = self.coarse_sky(strokes);
        let mut region: Vec<bool> = coarse
            .resized(self.width, self.height)
            .data
            .into_iter()
            .map(|value| value >= 0.5)
            .collect();
        let speck = ((self.width * self.height) as f32 * SPECK_SHARE) as usize;
        without_specks(&mut region, self.width, self.height, speck.max(MIN_SPECK));
        let forced = raster(strokes, self.width, self.height, 1.0, |paint| {
            paint != Paint::Refine
        });
        for (inside, paint) in region.iter_mut().zip(&forced) {
            match paint {
                Some(Paint::Sky) => *inside = true,
                Some(_) => *inside = false,
                None => {}
            }
        }
        self.refine(&mut region, strokes, &forced);
        without_specks(
            &mut region,
            self.width,
            self.height,
            speck.max(MIN_SPECK) / 2,
        );
        Detection {
            width: self.width,
            height: self.height,
            sky: region,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: usize = 400;
    const HEIGHT: usize = 300;

    struct Noise(u64);

    impl Noise {
        fn next(&mut self) -> f32 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.0 >> 40) as f32 / (1u64 << 24) as f32) - 0.5
        }
    }

    /// A night picture whose ground begins at `horizon(x)`: a smooth sky with a glow and stars
    /// above, `ground_light` with rough texture below.
    fn scene(
        horizon: impl Fn(usize) -> f32,
        sky_light: f32,
        ground_light: f32,
    ) -> (Vec<u8>, Vec<bool>) {
        let mut noise = Noise(7);
        let mut rgb = Vec::with_capacity(WIDTH * HEIGHT * 3);
        let mut truth = Vec::with_capacity(WIDTH * HEIGHT);
        let stars: Vec<(usize, usize)> = (0..260)
            .map(|_| {
                (
                    ((noise.next() + 0.5) * WIDTH as f32) as usize % WIDTH,
                    ((noise.next() + 0.5) * HEIGHT as f32) as usize % HEIGHT,
                )
            })
            .collect();
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let sky = (y as f32) < horizon(x);
                truth.push(sky);
                let glow = 1.0 + 0.5 * y as f32 / HEIGHT as f32;
                let level = if sky {
                    sky_light * glow
                } else {
                    ground_light * (1.0 + 0.9 * noise.next())
                };
                let value = level + 0.01 * noise.next();
                let tint = if sky {
                    [0.8, 0.9, 1.15]
                } else {
                    [1.0, 1.0, 1.0]
                };
                for channel in tint {
                    rgb.push((value * channel * 255.0).clamp(0.0, 255.0) as u8);
                }
            }
        }
        for (x, y) in stars {
            if truth[y * WIDTH + x] {
                for channel in 0..3 {
                    rgb[(y * WIDTH + x) * 3 + channel] = 255;
                }
            }
        }
        (rgb, truth)
    }

    fn agreement(found: &[bool], truth: &[bool]) -> f32 {
        found.iter().zip(truth).filter(|(a, b)| a == b).count() as f32 / truth.len() as f32
    }

    fn analyse(rgb: &[u8]) -> Sky {
        Sky::analyze(&Picture {
            width: WIDTH,
            height: HEIGHT,
            rgb,
        })
        .unwrap()
    }

    fn hills(x: usize) -> f32 {
        190.0 + 25.0 * (x as f32 / 40.0).sin() + 10.0 * (x as f32 / 9.0).sin()
    }

    #[test]
    fn dark_ground_under_a_brighter_sky_is_found() {
        let (rgb, truth) = scene(hills, 0.22, 0.04);
        let found = analyse(&rgb).detect(&[]);
        assert!(
            agreement(&found.sky, &truth) > 0.97,
            "{}",
            agreement(&found.sky, &truth)
        );
    }

    #[test]
    fn bright_ground_under_a_darker_sky_is_found() {
        let (rgb, truth) = scene(hills, 0.06, 0.4);
        let found = analyse(&rgb).detect(&[]);
        assert!(
            agreement(&found.sky, &truth) > 0.95,
            "{}",
            agreement(&found.sky, &truth)
        );
    }

    #[test]
    fn a_picture_of_only_sky_is_all_sky() {
        let (rgb, _) = scene(|_| HEIGHT as f32, 0.2, 0.05);
        let found = analyse(&rgb).detect(&[]);
        assert!(found.sky_fraction() > 0.99);
    }

    #[test]
    fn strokes_of_ground_and_sky_are_obeyed() {
        let (rgb, _) = scene(hills, 0.22, 0.04);
        let sky = analyse(&rgb);
        let strokes = [
            Stroke {
                paint: Paint::Ground,
                radius: 20.0,
                points: vec![[100.0, 50.0], [200.0, 50.0]],
            },
            Stroke {
                paint: Paint::Sky,
                radius: 20.0,
                points: vec![[300.0, 270.0]],
            },
        ];
        let found = sky.detect(&strokes);
        assert!(!found.sky[50 * WIDTH + 150]);
        assert!(found.sky[270 * WIDTH + 300]);
        assert!(found.sky[10 * WIDTH + 10]);
    }

    #[test]
    fn the_border_is_found_again_near_a_refine_stroke() {
        let (rgb, truth) = scene(hills, 0.22, 0.04);
        let sky = analyse(&rgb);
        let plain = sky.detect(&[]);
        let refined = sky.detect(&[Stroke {
            paint: Paint::Refine,
            radius: 40.0,
            points: vec![[100.0, 200.0], [300.0, 200.0]],
        }]);
        assert_eq!(plain.width, refined.width);
        let near = |found: &Detection| {
            let rows = 160..240;
            let pick = |values: &[bool]| -> Vec<bool> {
                rows.clone()
                    .flat_map(|y| (100..300).map(move |x| values[y * WIDTH + x]))
                    .collect()
            };
            agreement(&pick(&found.sky), &pick(&truth))
        };
        assert!(
            near(&refined) >= near(&plain) - 1e-6,
            "{} vs {}",
            near(&refined),
            near(&plain)
        );
        assert!(plain.sky != refined.sky, "refining changed nothing");
    }

    #[test]
    fn a_tiny_picture_is_refused() {
        let rgb = vec![0u8; 8 * 8 * 3];
        assert!(matches!(
            Sky::analyze(&Picture {
                width: 8,
                height: 8,
                rgb: &rgb
            }),
            Err(SkyError::TooSmall)
        ));
        assert!(matches!(
            Sky::analyze(&Picture {
                width: 100,
                height: 100,
                rgb: &rgb
            }),
            Err(SkyError::BadPicture)
        ));
    }
}
