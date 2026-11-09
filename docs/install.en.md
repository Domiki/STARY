# Install

## Download

Get the file for your system from the [latest release](https://github.com/Domiki/STARY/releases).

| System | File |
|---|---|
| macOS, Apple silicon (M1 or later) | `.dmg` |
| Windows 10 / 11, 64-bit | installer |

Intel Macs and Linux are not supported yet.

## The first time you open it { #first-open }

Pre-release builds are not signed with a paid developer certificate, so your system warns you once.

=== "macOS"

    Open the `.dmg` and drag STARY to **Applications**. If macOS refuses to open the app, right-click it and
    choose **Open**, or allow it under *System Settings → Privacy & Security → Open Anyway*. If it says the
    app is damaged, remove the quarantine flag:

    ```
    xattr -cr /Applications/STARY.app
    ```

=== "Windows"

    Run the installer. If Windows SmartScreen appears, choose **More info → Run anyway**. The WebView2 runtime
    is required; it is already part of Windows 11.

## Requirements

- **Memory**: 16 GB or more is recommended for 24–26 megapixel RAW files. The *Memory budget* in the
  Operation panel limits how much STARY uses.
- **GPU**: optional. Metal, DirectX 12 or Vulkan speeds up stacking.
- **Disk**: a 26-megapixel result is about 300 MB (32-bit float TIFF).

## Build from source

You need Rust (the version is pinned in `rust-toolchain.toml`) and Node.js.

```
git clone https://github.com/Domiki/STARY.git
cd STARY
npm install
npm --prefix ui install
npx tauri build
```

On Windows you also need the Visual Studio Build Tools (C++ workload) and LLVM, which compiles LibRaw
(see `.cargo/config.toml`). The installers are written to `target/release/bundle`.
