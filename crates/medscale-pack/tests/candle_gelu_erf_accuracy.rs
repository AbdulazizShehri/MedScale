//! The vendored candle-core computes f32 GELU (erf form) with a single-precision
//! erf for speed (vendor/candle-core-0.9.1/MEDSCALE_PATCH.md). It must stay
//! within f32 noise of the double-precision path the published crate used.
use candle_core::{DType, Device, Tensor};

#[test]
fn f32_gelu_erf_matches_the_double_precision_path() {
    let n = 400_001;
    let xs: Vec<f64> = (0..n)
        .map(|i| -10.0 + 20.0 * i as f64 / (n - 1) as f64)
        .collect();
    let x64 = Tensor::from_vec(xs.clone(), n, &Device::Cpu).unwrap();
    let reference: Vec<f64> = x64.gelu_erf().unwrap().to_vec1().unwrap();
    let fast: Vec<f32> = x64
        .to_dtype(DType::F32)
        .unwrap()
        .gelu_erf()
        .unwrap()
        .to_vec1()
        .unwrap();
    let mut worst = 0f64;
    for ((x, r), f) in xs.iter().zip(&reference).zip(&fast) {
        let err = (f64::from(*f) - r).abs();
        // Absolute error near zero, relative error elsewhere.
        let bound = 1e-6_f64.max(4e-7 * x.abs());
        assert!(
            err <= bound,
            "gelu_erf({x}) = {f}, expected {r} (error {err:e})"
        );
        worst = worst.max(err);
    }
    assert!(worst > 0.0, "the single-precision path is in use");
}

#[test]
fn f32_gelu_tanh_matches_the_double_precision_path() {
    // DistilBERT in candle-transformers uses the tanh form; it gets the same
    // single-precision treatment (tanh).
    let n = 400_001;
    let xs: Vec<f64> = (0..n)
        .map(|i| -12.0 + 24.0 * i as f64 / (n - 1) as f64)
        .collect();
    let x64 = Tensor::from_vec(xs.clone(), n, &Device::Cpu).unwrap();
    let reference: Vec<f64> = x64.gelu().unwrap().to_vec1().unwrap();
    let fast: Vec<f32> = x64
        .to_dtype(DType::F32)
        .unwrap()
        .gelu()
        .unwrap()
        .to_vec1()
        .unwrap();
    for ((x, r), f) in xs.iter().zip(&reference).zip(&fast) {
        let err = (f64::from(*f) - r).abs();
        let bound = 1.5e-6_f64.max(3e-7 * x.abs());
        assert!(
            err <= bound,
            "gelu({x}) = {f}, expected {r} (error {err:e})"
        );
    }
}

#[test]
fn f32_gelu_erf_handles_extremes() {
    let x = Tensor::new(
        &[f32::NEG_INFINITY, -1e30, -0.0, 0.0, 1e30, f32::INFINITY],
        &Device::Cpu,
    )
    .unwrap();
    let y: Vec<f32> = x.gelu_erf().unwrap().to_vec1().unwrap();
    assert_eq!(y[2], 0.0);
    assert_eq!(y[3], 0.0);
    assert!((y[4] / 1e30 - 1.0).abs() < 1e-6);
    assert_eq!(y[5], f32::INFINITY);
    // Large negative inputs give ~0 (bounded by the clamp), never NaN.
    assert!(y[1].abs() < 1e25 && y[1].is_finite());
}
