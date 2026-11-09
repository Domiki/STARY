# Files and sessions

## Adding files

Drag files or folders onto a role row, or click the **+** next to it and pick files. STARY reads their
details (format, size, exposure, ISO) and asks you to confirm. The status bar shows the progress while many
files are read. Files that cannot be added are listed with the reason, for example a duplicate, a size
that differs from the other files of the session or an unsupported format.

You can remove files from the list with **Delete** or right-click → **Remove … from list**. This only removes
them from STARY's list; the files on disk are not touched.

## Sessions

A session groups frames that belong together: Light frames with their own Dark, Flat, Dark Flat and Bias
frames. Use one session per night, per camera setup or per exposure.

- **+** in the Files header adds a session. Double-click a session name to rename it. The **×** removes it.
- Calibration is applied **per session**: each session uses its own Dark, Flat and Bias.
- In Stacking mode, every session with Light frames is stacked together. See
  [Deep-sky stacking](deep-sky.md#several-sessions).

## Roles

| Role | Frames |
|---|---|
| **Light** | The photographs of the sky |
| **Dark** | Taken with the lens cap on, same exposure and ISO as the Lights |
| **Flat** | Evenly lit frames, to correct vignetting and dust |
| **Dark Flat** | Dark frames with the exposure of the Flats |
| **Bias** | The shortest possible exposure with the lens cap on |

## Master frames

If you already have a master dark, flat or bias, add it as the only file of that role, then right-click it and
choose **Set as master**. A **MASTER** badge appears, and STARY uses it as it is instead of combining frames.
**Clear master** undoes this.

## The representative image

Each session has a representative Light image, marked with a star. It is the frame the others are aligned
to and the picture on which the sky mask is made. The first Light is the representative by default; right-click
another Light and choose **Set as representative image** to change it.

## Warnings and badges

A warning icon next to a file means something may not match, and its tooltip tells what:

- The **exposure** or **ISO/Gain** differs from the representative Light (or, inside one role, from the first
  file).
- The **file format** differs from the rest of the session.
- Calibration with **JPEG/PNG** is less accurate.

After a stack, Light frames that were not used get a **Left out** badge; hover it to see why (for example
*Blurrier than the other frames*).
