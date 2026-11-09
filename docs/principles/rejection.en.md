# Outlier rejection

A satellite, an airplane or a cosmic ray ruins a pixel in one frame. If you only average, the stripe stays
as a faint line. Rejection finds such values and leaves them out.

## Sigma clipping

For every pixel and color, STARY looks at the values of all frames at that place:

1. **First pass.** Add the values up to get the mean and the spread (standard deviation, *σ*) of each pixel.
2. **Second pass.** Add up again, this time only the values that lie within *low* σ below and *high* σ above the
   mean (3 and 3 by default), and divide by their number.

A value that is much brighter than the rest, such as a satellite, falls outside the limits and is rejected.
**Average** skips this and uses every value.

The frames are produced twice, once per pass, instead of being kept in memory, so the memory used does not depend
on the number of frames (for 26-megapixel frames about 1 GB for the running totals). This is what lets a thousand
frames be stacked.

## Limits to keep in mind

- **Few frames.** With fewer than about eleven frames, a single outlier cannot be far enough from the mean (the
  mean is pulled toward it), so 3σ clipping may not catch it. Use more frames or a lower limit.
- **Very smooth data.** If the frames are nearly identical, σ is tiny and ordinary noise can be rejected;
  raise the limits slightly.

## On the GPU

The sums of the two passes run on the GPU when it is selected and the totals fit in its memory. On the CPU the result is
the same within rounding error (tested to within a small fraction of a level).
