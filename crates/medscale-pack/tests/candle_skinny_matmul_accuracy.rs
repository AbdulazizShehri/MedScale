//! The vendored candle-core multiplies few-row f32 inputs by transposed
//! weights with its own kernel (vendor/candle-core-0.9.1/MEDSCALE_PATCH.md,
//! item 6). `Linear` on short token sequences takes that path; it must equal a
//! double-precision reference within f32 summation noise, and match gemm
//! outside it.
use candle_core::{DType, Device, Module, Tensor};

fn lcg(seed: &mut u64) -> f32 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 40) as f32 / (1u64 << 24) as f32) - 0.5
}

fn reference(x: &[f32], w: &[f32], bias: &[f32], m: usize, n: usize, k: usize) -> Vec<f64> {
    (0..m * n)
        .map(|idx| {
            let (i, j) = (idx / n, idx % n);
            bias[j] as f64
                + (0..k)
                    .map(|t| x[i * k + t] as f64 * w[j * k + t] as f64)
                    .sum::<f64>()
        })
        .collect()
}

#[test]
fn linear_on_short_sequences_matches_a_double_precision_reference() {
    let device = Device::Cpu;
    let mut seed = 7u64;
    // (tokens, out, in): BERT-base projections and FFN, odd sizes, tails.
    for &(m, n, k) in &[
        (1, 768, 768),
        (7, 3072, 768),
        (16, 768, 3072),
        (33, 130, 515),
        (64, 768, 768),
    ] {
        let x: Vec<f32> = (0..m * k).map(|_| lcg(&mut seed)).collect();
        let w: Vec<f32> = (0..n * k).map(|_| lcg(&mut seed)).collect();
        let b: Vec<f32> = (0..n).map(|_| lcg(&mut seed)).collect();
        let linear = candle_nn::Linear::new(
            Tensor::from_vec(w.clone(), (n, k), &device).unwrap(),
            Some(Tensor::from_vec(b.clone(), n, &device).unwrap()),
        );
        let input = Tensor::from_vec(x.clone(), (1, m, k), &device).unwrap();
        let got: Vec<f32> = linear
            .forward(&input)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();
        let want = reference(&x, &w, &b, m, n, k);
        let tol = 2e-6 * (k as f64).sqrt();
        for (g, r) in got.iter().zip(&want) {
            assert!((f64::from(*g) - r).abs() <= tol, "{m}x{n}x{k}: {g} vs {r}");
        }
    }
}

#[test]
fn larger_inputs_and_other_layouts_still_agree() {
    // 65 rows (gemm path) and a contiguous non-transposed rhs (gemm path)
    // against the same products through the fast path.
    let device = Device::Cpu;
    let mut seed = 11u64;
    let (k, n) = (256usize, 512usize);
    let w: Vec<f32> = (0..n * k).map(|_| lcg(&mut seed)).collect();
    let wt = Tensor::from_vec(w, (n, k), &device).unwrap();
    for m in [16usize, 65] {
        let x = Tensor::from_vec(
            (0..m * k).map(|_| lcg(&mut seed)).collect::<Vec<_>>(),
            (m, k),
            &device,
        )
        .unwrap();
        let fast = x.matmul(&wt.t().unwrap()).unwrap();
        let gemm = x.matmul(&wt.t().unwrap().contiguous().unwrap()).unwrap();
        let diff = (fast - gemm)
            .unwrap()
            .abs()
            .unwrap()
            .max_all()
            .unwrap()
            .to_dtype(DType::F64)
            .unwrap();
        assert!(diff.to_scalar::<f64>().unwrap() < 1e-4, "rows {m}");
    }
}
