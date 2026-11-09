# Settings

## The settings window

The settings button at the top right of the window opens it.

- **Compute device** — **CPU** or **GPU**. Below the buttons you see the number of CPU threads and the name of the
  GPU with its backend (Metal, DirectX 12 or Vulkan). When a GPU is found it is selected for you. The GPU makes stacking
  faster; the CPU always works. If a picture does not fit in the GPU's memory, or the GPU fails, choose the CPU.
- **Memory budget** — the share of the free memory STARY may use (10–80 %, 50 % by default). A larger budget lets more
  frames be processed at the same time.

## Other choices

- **File reading** (Calibration section): *Exact* or *Fast*, see [Calibration](calibration.md).
- **Demosaic quality** (Mode section): turns the raw sensor data into a color picture. *Fast* is good for
  most work. *High* uses a slower method (for X-Trans sensors a three-pass algorithm) and can take many times
  longer; in tests it did not change the noise of a stack.
- **Language**: the settings button at the top right of the window. *System default* follows your system; English and
  Korean are available.

## Updates

When STARY starts, it looks at the list of releases on GitHub. If a newer version has come out, a window shows its release notes and a button that takes you to the download; **Close** simply keeps the version you have. This is the only thing STARY asks the internet for, and nothing about you or your files is sent. You can turn it off with **Look for a new version at startup** in the settings (the button at the top right), and look yourself at any time with **Check for updates** in *Help → About STARY*.

!!! note
    The device, memory budget and file reading choices return to their defaults when STARY is restarted.
