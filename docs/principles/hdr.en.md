# HDR merge

Bright parts of the sky, such as the core of a nebula or a bright star, **saturate** in a long exposure: the sensor
reaches its maximum and the detail is gone. A short exposure keeps that detail but is too noisy for the faint parts.
HDR merge takes each from the exposure that records it best.

## Steps

1. **Group.** The sessions are grouped by exposure time and ISO. Sessions with the same exposure form one group.
2. **Align everything together** on the representative image, as in a normal stack, so that all groups end up
   in the same coordinates.
3. **Stack each group on its own.** Brightness normalization works only inside a group; the groups
   are not matched to each other, because their brightness differs on purpose.
4. **Find where the longest exposure is full.** Where the largest of the three colors is close to the maximum
   value (a soft transition from 85 % to 97 %), the long stack may be cut off.
5. **Bring the short exposure to the same scale.** The exposure time times the ISO gives a first ratio between
   the two. STARY refines it with a straight-line fit (`long ≈ a · short + b`, separately for each color) over pixels
   that are well measured in both (neither too dark nor near saturation). The fit may differ from the first ratio by at
   most a factor of two.
6. **Merge.** Where the long exposure is full, its value is replaced by the scaled short value, with a soft
   transition. All three colors switch together, so the colors of bright stars do not shift.
7. **More exposures.** With three or more exposures, the same is done from the longest to the shortest.

## The result

The result is linear and in the units of the longest exposure. The bright parts can be above 1.0, which a
32-bit float file keeps. Viewers that expect values up to 1.0 show them clipped; for such viewers save
with *Auto stretch* or *Gamma*.

## Limits

- Every session must hold frames of one exposure and ISO.
- A short exposure has few stars, so its frames can fail to align; failed frames are left out and marked.
- Ground separation is not used with HDR merge.
- This feature is experimental and has been tested on synthetic data.
