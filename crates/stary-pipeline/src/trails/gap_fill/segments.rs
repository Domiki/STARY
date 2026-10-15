use std::ops::Range;

/// How the lights of a run are cut into stretches that are merged separately. The motion of the
/// stars may differ from one stretch to the next, and one picture can be filled along one motion
/// only. The last light has a stretch of its own: there is no next light to fill towards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segments {
    frames: usize,
    length: usize,
}

impl Segments {
    pub fn new(frames: usize, preferred_length: usize, max_stretches: usize) -> Self {
        let regular = frames.saturating_sub(1);
        let stretches = regular
            .div_ceil(preferred_length.max(1))
            .clamp(1, max_stretches.max(1));
        Self {
            frames,
            length: regular.div_ceil(stretches).max(1),
        }
    }

    fn regular(&self) -> usize {
        self.frames.saturating_sub(1).div_ceil(self.length)
    }

    /// The stretches, with the one of the last light after the others.
    pub fn count(&self) -> usize {
        self.regular() + usize::from(self.frames > 0)
    }

    pub fn of(&self, index: usize) -> usize {
        if index + 1 >= self.frames {
            self.regular()
        } else {
            index / self.length
        }
    }

    pub fn is_last(&self, stretch: usize) -> bool {
        stretch == self.regular()
    }

    /// The pairs of a stretch whose motion is measured, named by their first light: spread evenly
    /// over the stretch, and no more than `count`. A few pairs tell the motion of a stretch.
    pub fn sampled_pairs(&self, stretch: usize, count: usize) -> Vec<usize> {
        if self.is_last(stretch) {
            return Vec::new();
        }
        let lights = self.lights(stretch);
        let (start, len) = (lights.start, lights.len());
        if len <= count {
            return lights.collect();
        }
        (0..count).map(|step| start + step * len / count).collect()
    }

    /// The lights of a stretch, whose pairs with the next light give its motion.
    pub fn lights(&self, stretch: usize) -> Range<usize> {
        if self.is_last(stretch) {
            return self.frames.saturating_sub(1)..self.frames;
        }
        let start = stretch * self.length;
        start..(start + self.length).min(self.frames - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_light_has_a_stretch_of_its_own() {
        let segments = Segments::new(10, 100, 16);
        assert_eq!(segments.count(), 2);
        assert_eq!(segments.of(0), 0);
        assert_eq!(segments.of(8), 0);
        assert_eq!(segments.of(9), 1);
        assert!(segments.is_last(1) && !segments.is_last(0));
        assert_eq!(segments.lights(0), 0..9);
        assert_eq!(segments.lights(1), 9..10);
    }

    #[test]
    fn long_runs_are_cut_into_stretches_of_the_preferred_length() {
        let segments = Segments::new(251, 100, 16);
        assert_eq!(segments.count(), 4);
        assert_eq!(segments.of(83), 0);
        assert_eq!(segments.of(84), 1);
        assert_eq!(segments.lights(2), 168..250);
        assert_eq!(segments.of(250), 3);
    }

    #[test]
    fn the_stretches_are_of_even_length() {
        let segments = Segments::new(102, 100, 16);
        assert_eq!(segments.count(), 3);
        assert_eq!(segments.lights(0), 0..51);
        assert_eq!(segments.lights(1), 51..101);
    }

    #[test]
    fn every_light_belongs_to_exactly_one_stretch() {
        let segments = Segments::new(53, 10, 16);
        let mut seen = vec![0; 53];
        for stretch in 0..segments.count() {
            for index in segments.lights(stretch) {
                seen[index] += 1;
                assert_eq!(segments.of(index), stretch);
            }
        }
        assert!(seen.iter().all(|&count| count == 1));
    }

    #[test]
    fn a_few_pairs_spread_over_the_stretch_are_measured() {
        let segments = Segments::new(201, 100, 16);
        assert_eq!(segments.sampled_pairs(0, 4), vec![0, 25, 50, 75]);
        assert_eq!(segments.sampled_pairs(1, 4), vec![100, 125, 150, 175]);
        assert_eq!(segments.sampled_pairs(0, 500), (0..100).collect::<Vec<_>>());
        assert!(segments.sampled_pairs(2, 4).is_empty());
    }

    #[test]
    fn there_are_never_more_than_a_handful_of_stretches() {
        assert!(Segments::new(5000, 10, 16).count() <= 17);
        assert_eq!(Segments::new(40, 1, 64).count(), 40);
    }

    #[test]
    fn a_single_light_is_its_own_stretch() {
        let segments = Segments::new(1, 100, 16);
        assert_eq!(segments.count(), 1);
        assert!(segments.is_last(0));
    }
}
