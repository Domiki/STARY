use rsraw::RawImage;

use crate::error::RawError;

pub struct SensorBuffer<'a> {
    pub samples: &'a [u16],
    pub pitch: usize,
    pub rows: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct ActiveArea {
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
}

pub fn active_area(raw: &RawImage) -> ActiveArea {
    let sizes = &raw.as_ref().rawdata.sizes;
    ActiveArea {
        left: usize::from(sizes.left_margin),
        top: usize::from(sizes.top_margin),
        width: usize::from(sizes.width),
        height: usize::from(sizes.height),
    }
}

// ------- the only place in the project that dereferences a LibRaw pointer -------
pub fn sensor_buffer(raw: &RawImage) -> Result<SensorBuffer<'_>, RawError> {
    let rawdata = &raw.as_ref().rawdata;
    let pointer = rawdata.raw_image;
    let pitch = rawdata.sizes.raw_pitch as usize / size_of::<u16>();
    let rows = usize::from(rawdata.sizes.raw_height);
    let length = pitch.checked_mul(rows).ok_or(RawError::BadGeometry)?;
    if pointer.is_null() {
        return Err(RawError::NoSensorBuffer);
    }
    if length == 0 {
        return Err(RawError::BadGeometry);
    }
    // SAFETY: after a successful `unpack`, LibRaw owns a `raw_pitch * raw_height` byte buffer at
    // `raw_image`, 2-byte aligned. The returned slice borrows `raw`, so the buffer cannot be
    // freed or reallocated while the slice is alive, and nothing writes to it meanwhile.
    let samples = unsafe { std::slice::from_raw_parts(pointer, length) };
    Ok(SensorBuffer {
        samples,
        pitch,
        rows,
    })
}

pub struct SensorBufferMut<'a> {
    pub samples: &'a mut [u16],
    pub pitch: usize,
    pub rows: usize,
}

pub fn sensor_buffer_mut(raw: &mut RawImage) -> Result<SensorBufferMut<'_>, RawError> {
    let rawdata = &raw.as_ref().rawdata;
    let pointer = rawdata.raw_image;
    let pitch = rawdata.sizes.raw_pitch as usize / size_of::<u16>();
    let rows = usize::from(rawdata.sizes.raw_height);
    let length = pitch.checked_mul(rows).ok_or(RawError::BadGeometry)?;
    if pointer.is_null() {
        return Err(RawError::NoSensorBuffer);
    }
    if length == 0 {
        return Err(RawError::BadGeometry);
    }
    // SAFETY: same buffer as in `sensor_buffer`. The exclusive borrow of `raw` guarantees that no
    // other slice into it exists and that LibRaw does not touch it while the slice is alive.
    let samples = unsafe { std::slice::from_raw_parts_mut(pointer, length) };
    Ok(SensorBufferMut {
        samples,
        pitch,
        rows,
    })
}
