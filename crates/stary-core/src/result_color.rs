/// How the camera's colors turn into linear sRGB: each row is the share of the camera's red, green
/// and blue that make one sRGB primary.
pub type CameraToSrgb = [[f32; 3]; 3];

/// What the three values of a pixel of a result mean.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ResultColor {
    /// Linear light with the primaries of sRGB.
    #[default]
    LinearSrgb,
    /// The colors the camera recorded, white balanced and linear (stacks keep them so as not to add
    /// noise to the weaker channels).
    Camera(CameraToSrgb),
}
