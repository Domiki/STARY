# Alignment and normalization

Stacking works only if the stars of every frame lie on top of each other and the frames have the same brightness.

## Finding stars

STARY looks for stars in a half-size copy of the picture (each 2 × 2 block is averaged, which also lowers noise).
For each star it measures the position, brightness, width (FWHM) and roundness. With a ground mask, only
stars in the sky are used.

## Matching

Two frames show the same stars in different places. STARY forms triangles of nearby stars, whose shape does not
change with shift or rotation, and finds the triangles that appear in both frames. A robust fit (RANSAC) then
finds the transform that brings the matching stars together. It starts loose (10 pixels) and tightens to 1.5 pixels,
refining the transform each round. The **alignment model** sets how general the transform may be:

| Model | Allows |
|---|---|
| **Similarity** | shift and rotation (default; enough for a tripod or a tracker) |
| **Affine** | also stretching and shear |
| **Homography** | also perspective, for wide fields and lens distortion |

A frame that cannot be matched, because it has too few stars or the stars do not agree with the reference, is left
out and marked.

## Moving the frames

Every pixel of the result is looked up in the frame through the inverse transform and interpolated
(*Bilinear*, *Bicubic* or *Lanczos-3*). Sharper kernels keep more detail; the results are limited so that they do not ring
around bright stars.

## Brightness normalization

Frames differ in brightness: the sky glows more as the night goes on, the Moon rises, clouds pass. For each frame
and each color STARY measures the sky level and its spread (noise), and then adds and scales the frame so that
both match those of the reference frame. Without this, stacking would blend frames of different brightness.

## Quality filter

When **Skip poor frames** is on, every frame is compared with the *typical* frame of its session (the median of each
measure): sky brightness, noise, number of stars, star width and roundness. A frame that is much brighter, noisier,
blurrier or more trailed, or that has far fewer stars, is left out. The typical frame is used instead of the best
frame so that a whole night of mediocre frames is not thrown away. The frame you chose as the representative
is never left out.
