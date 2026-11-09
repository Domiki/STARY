# Troubleshooting

## The result looks very dark

The result is linear, which looks dark on a screen. STARY stretches it only for its own viewer. In an editing
program stretch the 32-bit TIFF, or export with **Save as…** and choose *Gamma* or *Auto stretch*.

## The GPU is not found, or the GPU fails

- Without a GPU, choose **CPU** in the settings.
- If the picture is too large for the GPU's memory, or the message says *The GPU could not do the work*,
  choose **CPU** and run again.
- Update your graphics driver. On Windows STARY uses DirectX 12 or Vulkan.

## Fewer frames were stacked than I added

The status bar tells how many could not be aligned and how many were left out for quality. Look at the **Left out**
badges in the file list; the tooltip gives the reason.

- *Not enough stars to align* or *Its stars did not match the reference frame* — the frame is too blurred, too
  dark or shows another part of the sky. Check the frame, or choose another representative image.
- *The sky is much brighter*, *Blurrier*, *Trailed* — the quality filter. Turn off **Skip poor frames** if you want
  them anyway.

## "Fewer than two frames could be aligned"

Check that all frames show the same part of the sky and that the stars are visible (not clouded out). With a
ground mask, the sky of the mask needs stars; check the mask.

## "Few stars in the sky" before running

The sky of your mask holds fewer than 10 stars. Check the mask (the ground may cover too much sky) or run anyway.

## It is slow

- Use the GPU if you have one (in the settings).
- *Demosaic quality: High* takes many times longer than *Fast*.
- A large memory budget helps; a small one lets fewer frames run at the same time.
- Reading from a slow drive: copy the files to an internal drive, or choose **File reading: Fast** for
  calibration frames.

## STARY says the app cannot be opened (macOS), or SmartScreen appears (Windows)

See [Install](../install.md#first-open).

## Report a problem

Please open an issue on [GitHub](https://github.com/Domiki/STARY/issues) with your system, STARY's version, what
you did and, if you can, an example file.
