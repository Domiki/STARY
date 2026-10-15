// ------- pipeline constants: 0 bilinear, 1 Catmull-Rom, 2 Lanczos3; samples read per row or column -------
override KERNEL: u32 = 1u;
override TAPS: u32 = 4u;

const PI: f32 = 3.14159265358979;
const FULL_SCALE: f32 = 65535.0;
const UNMOVED: u32 = 1u;
const CLIPPED: u32 = 2u;

// ------- state of one sample: Welford mean and squared distances, offsets of kept values, counts (covered | kept << 16) -------
struct Sample {
    mean: f32,
    squares: f32,
    offsets: f32,
    counts: u32,
}

struct Params {
    row0: vec4<f32>,
    row1: vec4<f32>,
    row2: vec4<f32>,
    gain: vec4<f32>,
    offset: vec4<f32>,
    clipping: vec4<f32>,
    width: u32,
    height: u32,
    plane_words: u32,
    flags: u32,
}

@group(0) @binding(0) var<storage, read> frame: array<u32>;
@group(0) @binding(1) var<storage, read_write> sky0: array<Sample>;
@group(0) @binding(2) var<storage, read_write> sky1: array<Sample>;
@group(0) @binding(3) var<storage, read_write> sky2: array<Sample>;
@group(0) @binding(4) var<uniform> p: Params;
@group(0) @binding(5) var<storage, read_write> picture: array<f32>;

fn load_sample(plane: u32, x: i32, y: i32) -> f32 {
    let cx = u32(clamp(x, 0, i32(p.width) - 1));
    let cy = u32(clamp(y, 0, i32(p.height) - 1));
    let index = cy * p.width + cx;
    let word = frame[plane * p.plane_words + (index >> 1u)];
    return f32((word >> ((index & 1u) * 16u)) & 0xffffu);
}

fn catmull_rom(distance: f32) -> f32 {
    let x = abs(distance);
    if (x < 1.0) {
        return 1.5 * x * x * x - 2.5 * x * x + 1.0;
    }
    if (x < 2.0) {
        return -0.5 * x * x * x + 2.5 * x * x - 4.0 * x + 2.0;
    }
    return 0.0;
}

fn sinc(x: f32) -> f32 {
    if (abs(x) < 1e-6) {
        return 1.0;
    }
    let angle = PI * x;
    return sin(angle) / angle;
}

fn lanczos(distance: f32) -> f32 {
    let x = abs(distance);
    if (x >= 3.0) {
        return 0.0;
    }
    return sinc(x) * sinc(x / 3.0);
}

// ------- resampling -------
fn weights(fraction: f32) -> array<f32, 6> {
    var out = array<f32, 6>(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    let first = 1 - i32(TAPS / 2u);
    var total = 0.0;
    for (var tap = 0u; tap < TAPS; tap++) {
        let distance = f32(first + i32(tap)) - fraction;
        var weight = 0.0;
        if (KERNEL == 0u) {
            weight = 1.0 - abs(distance);
        } else if (KERNEL == 1u) {
            weight = catmull_rom(distance);
        } else {
            weight = lanczos(distance);
        }
        out[tap] = weight;
        total += weight;
    }
    if (KERNEL == 2u) {
        for (var tap = 0u; tap < TAPS; tap++) {
            out[tap] = out[tap] / total;
        }
    }
    return out;
}

fn resample(plane: u32, ix: i32, iy: i32, wx: array<f32, 6>, wy: array<f32, 6>) -> f32 {
    let first = 1 - i32(TAPS / 2u);
    var total = 0.0;
    for (var row = 0u; row < TAPS; row++) {
        var line = 0.0;
        for (var column = 0u; column < TAPS; column++) {
            line += wx[column] * load_sample(plane, ix + first + i32(column), iy + first + i32(row));
        }
        total += wy[row] * line;
    }
    if (KERNEL == 0u) {
        return total;
    }
    let a = load_sample(plane, ix, iy);
    let b = load_sample(plane, ix + 1, iy);
    let c = load_sample(plane, ix, iy + 1);
    let d = load_sample(plane, ix + 1, iy + 1);
    return clamp(total, min(min(a, b), min(c, d)), max(max(a, b), max(c, d)));
}

fn picture_values(column: u32, row: u32) -> vec4<f32> {
    if ((p.flags & UNMOVED) != 0u) {
        let x = i32(column);
        let y = i32(row);
        return vec4<f32>(load_sample(0u, x, y), load_sample(1u, x, y), load_sample(2u, x, y), 1.0);
    }
    let fx = f32(column);
    let fy = f32(row);
    let w = p.row2.x * fx + p.row2.y * fy + p.row2.z;
    let x = (p.row0.x * fx + p.row0.y * fy + p.row0.z) / w;
    let y = (p.row1.x * fx + p.row1.y * fy + p.row1.z) / w;
    if (!(x >= 0.0 && y >= 0.0 && x <= f32(p.width - 1u) && y <= f32(p.height - 1u))) {
        return vec4<f32>(0.0);
    }
    let floor_x = floor(x);
    let floor_y = floor(y);
    let wx = weights(x - floor_x);
    let wy = weights(y - floor_y);
    let ix = i32(floor_x);
    let iy = i32(floor_y);
    return vec4<f32>(
        resample(0u, ix, iy, wx, wy) * p.gain.x + p.offset.x,
        resample(1u, ix, iy, wx, wy) * p.gain.y + p.offset.y,
        resample(2u, ix, iy, wx, wy) * p.gain.z + p.offset.z,
        1.0,
    );
}

fn welford(sample: Sample, value: f32) -> Sample {
    let covered = (sample.counts & 0xffffu) + 1u;
    let delta = value - sample.mean;
    let mean = sample.mean + delta / f32(covered);
    return Sample(
        mean,
        sample.squares + delta * (value - mean),
        sample.offsets,
        (sample.counts & 0xffff0000u) | covered,
    );
}

fn keep_within_spread(sample: Sample, value: f32) -> Sample {
    let covered = sample.counts & 0xffffu;
    if (covered == 0u) {
        return sample;
    }
    let spread = sqrt(max(sample.squares / f32(covered), 0.0));
    let offset = value - sample.mean;
    if (offset >= -p.clipping.x * spread && offset <= p.clipping.y * spread) {
        return Sample(sample.mean, sample.squares, sample.offsets + offset, sample.counts + 0x10000u);
    }
    return sample;
}

// ------- first lay -------
@compute @workgroup_size(16, 16, 1)
fn statistics(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= p.width || id.y >= p.height) {
        return;
    }
    let values = picture_values(id.x, id.y);
    if (values.w == 0.0) {
        return;
    }
    let index = id.y * p.width + id.x;
    sky0[index] = welford(sky0[index], values.x);
    sky1[index] = welford(sky1[index], values.y);
    sky2[index] = welford(sky2[index], values.z);
}

// ------- second lay -------
@compute @workgroup_size(16, 16, 1)
fn clipped(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= p.width || id.y >= p.height) {
        return;
    }
    let values = picture_values(id.x, id.y);
    if (values.w == 0.0) {
        return;
    }
    let index = id.y * p.width + id.x;
    sky0[index] = keep_within_spread(sky0[index], values.x);
    sky1[index] = keep_within_spread(sky1[index], values.y);
    sky2[index] = keep_within_spread(sky2[index], values.z);
}

fn combined(sample: Sample) -> f32 {
    let covered = sample.counts & 0xffffu;
    let kept = sample.counts >> 16u;
    var value = 0.0;
    if ((p.flags & CLIPPED) != 0u && kept > 0u) {
        value = sample.mean + sample.offsets / f32(kept);
    } else if (covered > 0u) {
        value = sample.mean;
    }
    return max(value, 0.0) / FULL_SCALE;
}

// ------- result -------
@compute @workgroup_size(16, 16, 1)
fn finish(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= p.width || id.y >= p.height) {
        return;
    }
    let index = id.y * p.width + id.x;
    picture[index * 3u] = combined(sky0[index]);
    picture[index * 3u + 1u] = combined(sky1[index]);
    picture[index * 3u + 2u] = combined(sky2[index]);
}
