# Supported files

| Format | Notes |
|---|---|
| **Camera RAW** | Read with LibRaw. Raw sensor values are used, so calibration is exact. Bayer and X-Trans (Fujifilm) sensors are handled. |
| **FITS** | Raw (Bayer or X-Trans pattern), color or monochrome. FITS from astronomy cameras and files made by Siril are read. |
| **TIFF**, **PNG**, **JPEG** | Read as ready-made pictures. Calibration works, but is less accurate because the values have been processed by a camera or editor. |

## Tested

- Camera RAW from Fujifilm X-Pro3 (X-Trans) and Sony (Bayer).
- FITS files and master frames made by Siril (X-Trans pattern).
- JPEG for star trails.

Files from astronomy cameras (Bayer FITS) and PNG/TIFF input have been tested on synthetic data only. Reports with
example files are very welcome.

## Output

Results are saved as 32-bit float TIFF automatically. See [Saving results](../guide/saving.md) for the other
formats.

## Rules inside a session

All frames of a session must have the same size and the same pixel layout. A file that does not fit is
refused when you add it, with the reason shown.
