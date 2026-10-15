//! Needs a real X-Trans RAF: STARY_TEST_RAF=/path/to/flat.RAF cargo test -p stary-raw -- --ignored
use stary_core::{CfaPattern, PixelLayout};
use stary_raw::decode_cfa;

fn position_means(samples: &[u16], width: usize, height: usize) -> [[f64; 6]; 6] {
    let mut sums = [[0f64; 6]; 6];
    let mut counts = [[0f64; 6]; 6];
    for row in 0..height {
        for column in 0..width {
            sums[row % 6][column % 6] += f64::from(samples[row * width + column]);
            counts[row % 6][column % 6] += 1.0;
        }
    }
    std::array::from_fn(|r| std::array::from_fn(|c| sums[r][c] / counts[r][c]))
}

#[test]
#[ignore = "needs a real RAF file"]
fn xtrans_pattern_matches_the_decoded_sensor_data() {
    let path = std::env::var("STARY_TEST_RAF").expect("set STARY_TEST_RAF");
    let frame = decode_cfa(&std::fs::read(path).unwrap()).unwrap();
    let PixelLayout::Cfa(CfaPattern::XTrans(matrix)) = frame.layout else {
        panic!("not X-Trans")
    };
    println!(
        "size {}x{} black {} maximum {}",
        frame.width, frame.height, frame.black, frame.maximum
    );

    let means = position_means(&frame.samples, frame.width, frame.height);
    let mut by_color: [Vec<f64>; 3] = Default::default();
    for r in 0..6 {
        for c in 0..6 {
            by_color[matrix[r][c] as usize].push(means[r][c]);
        }
    }
    let mut stats = Vec::new();
    for (name, values) in ["R", "G", "B"].iter().zip(&by_color) {
        let min = values.iter().cloned().fold(f64::MAX, f64::min);
        let max = values.iter().cloned().fold(f64::MIN, f64::max);
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        println!(
            "{name}: sites {:>2}, mean {mean:8.1}, range {min:8.1} .. {max:8.1}",
            values.len()
        );
        stats.push((min, max, mean));
    }
    let spread_within = stats
        .iter()
        .map(|(min, max, _)| max - min)
        .fold(0.0, f64::max);
    let mut means: Vec<f64> = stats.iter().map(|s| s.2).collect();
    means.sort_by(f64::total_cmp);
    let gap_between = (means[1] - means[0]).min(means[2] - means[1]);
    println!(
        "largest within-color spread {spread_within:.1}, smallest gap between colors {gap_between:.1}"
    );
    assert!(
        spread_within * 3.0 < gap_between,
        "colors are not separated: pattern phase is probably wrong"
    );
}

#[test]
#[ignore = "needs a real RAF file"]
fn demosaicing_the_unchanged_sensor_data_matches_libraw_directly() {
    use rsraw::{BIT_DEPTH_16, RawImage};
    use stary_raw::{DemosaicQuality, OutputColor, demosaic_cfa};

    let path = std::env::var("STARY_TEST_RAF").expect("set STARY_TEST_RAF");
    let bytes = std::fs::read(path).unwrap();
    let frame = decode_cfa(&bytes).unwrap();
    let via_stary = demosaic_cfa(
        &bytes,
        &frame.samples,
        DemosaicQuality::Fast,
        OutputColor::Srgb,
    )
    .unwrap();

    let mut raw = RawImage::open(&bytes).unwrap();
    raw.unpack().unwrap();
    {
        let params = &mut raw.as_mut().params;
        params.user_qual = 0;
        params.no_auto_bright = 1;
        params.gamm[0] = 1.0;
        params.gamm[1] = 1.0;
        params.use_camera_wb = 1;
        params.user_flip = 0;
    }
    let direct = raw.process::<BIT_DEPTH_16>().unwrap();
    assert_eq!(
        (via_stary.width, via_stary.height),
        (direct.width() as usize, direct.height() as usize)
    );
    assert_eq!(
        via_stary.samples,
        direct.to_vec(),
        "pasting the sensor data back changed the result"
    );
    println!("identical: {}x{} RGB16", via_stary.width, via_stary.height);
}
