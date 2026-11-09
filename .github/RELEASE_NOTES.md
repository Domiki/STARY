The first public build of STARY, a pre-release for macOS (Apple silicon) and Windows.

## What is in it

- Deep-sky stacking with star alignment, brightness normalization and sigma-clipping rejection; poor frames are left out and marked
- Star trails: Lighten, Comet and Gap Filling
- Calibration with Dark, Flat, Dark Flat and Bias frames or masters, one set per session
- Ground masking with automatic sky detection and a brush
- Several sessions in one stack, and an experimental HDR merge of different exposures
- GPU acceleration (Metal, DirectX 12, Vulkan)
- 32-bit float TIFF results; *Save as…* exports TIFF, PNG, JPEG and FITS in sRGB, Display P3 or Adobe RGB
- Camera RAW, FITS, JPEG, PNG and TIFF input
- Help menu with About, Report an issue and Documentation; a note when a newer version is out

## Install

- **macOS** (Apple silicon, M1 or later): open the `.dmg` and drag STARY to Applications. The build is not signed with a paid developer certificate: right-click the app and choose **Open** the first time, or run `xattr -cr /Applications/STARY.app` if macOS says it is damaged.
- **Windows** (64-bit, 10 or 11): run the installer. If SmartScreen appears, choose **More info → Run anyway**.

## Good to know

- Linux and Intel Macs are not supported yet.
- HDR merge and stacking several sessions have been tested on synthetic data and a few real sets only.

Manual: <https://stary.readthedocs.io/> · Website: <https://domiki.github.io/STARY/>
