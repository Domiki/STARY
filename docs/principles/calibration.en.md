# Calibration

## What goes wrong in a raw frame

- **Dark signal.** Even with no light, sensor pixels collect charge from heat. Long exposures and warm
  sensors collect more, and some pixels are hot.
- **Offset.** Each frame has a base value (the *black level*) from the electronics.
- **Uneven light.** Vignetting and dust shadows make the picture darker in places by a fixed factor.

## What STARY does

1. **Master frames.** The frames of a role are combined into one *master* by averaging with outlier
   rejection (a cosmic ray or a satellite in one frame does not enter the master). *Exact* reading takes
   two passes (find the mean and spread of every pixel, then average what lies within the limits); *Fast* reading
   takes one pass and drops the largest and smallest values of every pixel.
2. **Subtract.** The master dark (or the master bias when there is no dark) is subtracted from every
   Light. The Light keeps its own black level as the base.
3. **Divide by the flat.** The master flat, with its own offset removed (using the Dark Flat or Bias), is turned
   into a gain for each pixel and normalized for each color on its own, so the color of the flat's lamp does
   not tint the picture. Every Light is multiplied by that gain.
4. **Then demosaic.** All of this happens on the raw sensor values, before color is made.

## Good to know

- A dark has to match the exposure and ISO of the Lights. A dark of another exposure does not remove
  the dark signal correctly.
- When both Dark and Bias are present, the dark is used for the subtraction.
- Saving the master and calibrated frames lets you reuse them: a master can be added to a session as it is.
