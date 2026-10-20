//! MedScale performance patch (MEDSCALE_PATCH.md item 6): f32 matmul for
//! few-row inputs against a transposed contiguous weight, `dst[m][n] =
//! sum_k lhs[m][k] * w[n][k]`, which is how `Linear` multiplies short token
//! sequences. With few rows the multiply is bound by streaming the weight, so
//! each block of weight rows is read once and reused for every lhs row (held
//! in cache) instead of being packed per tile. Threads split the weight rows.
//!
//! x86_64 with AVX2 and FMA only. Elsewhere, including Apple Silicon where
//! gemm's NEON kernels measured 2.4-7.7x faster than a portable loop on the
//! same runner, every matmul stays on gemm.
#![cfg_attr(not(target_arch = "x86_64"), allow(dead_code))]

/// Largest row count served here; larger inputs use gemm.
pub(crate) const MAX_ROWS: usize = 64;
/// Smallest weight (n * k) worth the dispatch; tiny products use gemm.
pub(crate) const MIN_WEIGHT: usize = 16 * 1024;

struct SyncPtr(*mut f32);
unsafe impl Send for SyncPtr {}
unsafe impl Sync for SyncPtr {}

/// `lhs` rows start at `i * lhs_rs` (contiguous in k); `w` is `n` contiguous
/// rows of length `k`. Returns the `m x n` row-major product.
pub(crate) fn matmul_rows_by_wt(
    lhs: &[f32],
    lhs_rs: usize,
    m: usize,
    w: &[f32],
    n: usize,
    k: usize,
) -> Vec<f32> {
    let mut dst = vec![0f32; m * n];
    let threads = crate::utils::get_num_threads().max(1);
    // At least 32 weight rows per task so per-task overhead stays small.
    let tasks = threads.min(n.div_ceil(32)).max(1);
    let per_task = n.div_ceil(tasks).next_multiple_of(4);
    let out = SyncPtr(dst.as_mut_ptr());
    let run = |task: usize| {
        let n0 = task * per_task;
        let n1 = (n0 + per_task).min(n);
        if n0 < n1 {
            // SAFETY: tasks write disjoint column ranges [n0, n1) of `dst`.
            unsafe { block(lhs, lhs_rs, m, w, n, k, n0, n1, &out) }
        }
    };
    if tasks == 1 {
        run(0);
    } else {
        use rayon::prelude::*;
        (0..tasks).into_par_iter().for_each(run);
    }
    dst
}

/// Whether this kernel serves the current CPU.
pub(crate) fn available() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        static OK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *OK.get_or_init(|| {
            std::arch::is_x86_feature_detected!("avx2") && std::arch::is_x86_feature_detected!("fma")
        })
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// Callers check [`available`] first.
#[allow(clippy::too_many_arguments)]
unsafe fn block(
    lhs: &[f32],
    lhs_rs: usize,
    m: usize,
    w: &[f32],
    n: usize,
    k: usize,
    n0: usize,
    n1: usize,
    out: &SyncPtr,
) {
    #[cfg(target_arch = "x86_64")]
    x86::block(lhs, lhs_rs, m, w, n, k, n0, n1, out.0);
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (lhs, lhs_rs, m, w, n, k, n0, n1, out);
        unreachable!("few-row matmul is gated by available()");
    }
}

#[cfg(target_arch = "x86_64")]
mod x86 {
    use std::arch::x86_64::*;

    #[inline(always)]
    unsafe fn hsum(v: __m256) -> f32 {
        let lo = _mm256_castps256_ps128(v);
        let hi = _mm256_extractf128_ps(v, 1);
        let s = _mm_add_ps(lo, hi);
        let s = _mm_add_ps(s, _mm_movehl_ps(s, s));
        let s = _mm_add_ss(s, _mm_shuffle_ps(s, s, 0x55));
        _mm_cvtss_f32(s)
    }

    /// 2 lhs rows x 4 weight rows per step: 8 accumulators, 6 loads, 8 FMAs
    /// per 8 k. Weight rows of a 4-block stay in L1 across all lhs rows.
    #[target_feature(enable = "avx2,fma")]
    #[allow(clippy::too_many_arguments)]
    pub(super) unsafe fn block(
        lhs: &[f32],
        lhs_rs: usize,
        m: usize,
        w: &[f32],
        n: usize,
        k: usize,
        n0: usize,
        n1: usize,
        dst: *mut f32,
    ) {
        let kv = k / 8 * 8;
        let x = lhs.as_ptr();
        let wp = w.as_ptr();
        let mut j = n0;
        while j + 4 <= n1 {
            let w0 = wp.add(j * k);
            let (w1, w2, w3) = (w0.add(k), w0.add(2 * k), w0.add(3 * k));
            let mut i = 0;
            while i < m {
                let two = i + 1 < m;
                let x0 = x.add(i * lhs_rs);
                let x1 = if two { x.add((i + 1) * lhs_rs) } else { x0 };
                let mut a = [_mm256_setzero_ps(); 8];
                let mut t = 0;
                while t < kv {
                    let v0 = _mm256_loadu_ps(x0.add(t));
                    let v1 = _mm256_loadu_ps(x1.add(t));
                    let c0 = _mm256_loadu_ps(w0.add(t));
                    let c1 = _mm256_loadu_ps(w1.add(t));
                    let c2 = _mm256_loadu_ps(w2.add(t));
                    let c3 = _mm256_loadu_ps(w3.add(t));
                    a[0] = _mm256_fmadd_ps(v0, c0, a[0]);
                    a[1] = _mm256_fmadd_ps(v0, c1, a[1]);
                    a[2] = _mm256_fmadd_ps(v0, c2, a[2]);
                    a[3] = _mm256_fmadd_ps(v0, c3, a[3]);
                    a[4] = _mm256_fmadd_ps(v1, c0, a[4]);
                    a[5] = _mm256_fmadd_ps(v1, c1, a[5]);
                    a[6] = _mm256_fmadd_ps(v1, c2, a[6]);
                    a[7] = _mm256_fmadd_ps(v1, c3, a[7]);
                    t += 8;
                }
                let mut s = [0f32; 8];
                for (o, v) in s.iter_mut().zip(a) {
                    *o = hsum(v);
                }
                for t in kv..k {
                    let (a0, a1) = (*x0.add(t), *x1.add(t));
                    let (b0, b1, b2, b3) = (*w0.add(t), *w1.add(t), *w2.add(t), *w3.add(t));
                    s[0] += a0 * b0;
                    s[1] += a0 * b1;
                    s[2] += a0 * b2;
                    s[3] += a0 * b3;
                    s[4] += a1 * b0;
                    s[5] += a1 * b1;
                    s[6] += a1 * b2;
                    s[7] += a1 * b3;
                }
                let d0 = dst.add(i * n + j);
                for c in 0..4 {
                    *d0.add(c) = s[c];
                }
                if two {
                    let d1 = dst.add((i + 1) * n + j);
                    for c in 0..4 {
                        *d1.add(c) = s[4 + c];
                    }
                }
                i += 2;
            }
            j += 4;
        }
        // Remaining weight rows (fewer than 4).
        while j < n1 {
            let wr = std::slice::from_raw_parts(wp.add(j * k), k);
            for i in 0..m {
                let xr = std::slice::from_raw_parts(x.add(i * lhs_rs), k);
                *dst.add(i * n + j) = dot1(xr, wr);
            }
            j += 1;
        }
    }

    #[target_feature(enable = "avx2,fma")]
    unsafe fn dot1(a: &[f32], b: &[f32]) -> f32 {
        let k = a.len();
        let kv = k / 8 * 8;
        let mut acc = _mm256_setzero_ps();
        let mut t = 0;
        while t < kv {
            acc = _mm256_fmadd_ps(_mm256_loadu_ps(a.as_ptr().add(t)), _mm256_loadu_ps(b.as_ptr().add(t)), acc);
            t += 8;
        }
        let mut s = hsum(acc);
        for t in kv..k {
            s += a[t] * b[t];
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(lhs: &[f32], lhs_rs: usize, m: usize, w: &[f32], n: usize, k: usize) -> Vec<f64> {
        let mut out = vec![0f64; m * n];
        for i in 0..m {
            for j in 0..n {
                out[i * n + j] = (0..k).map(|t| lhs[i * lhs_rs + t] as f64 * w[j * k + t] as f64).sum();
            }
        }
        out
    }

    #[test]
    fn matches_a_double_precision_reference_across_shapes() {
        if !available() {
            return;
        }
        let mut seed = 0x2545F4914F6CDD1Du64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            ((seed >> 40) as f32 / (1u64 << 24) as f32) - 0.5
        };
        for &(m, n, k, pad) in &[(1, 37, 19, 0), (2, 64, 64, 3), (5, 129, 77, 0), (16, 768, 768, 0), (33, 300, 3072, 5), (64, 96, 9, 0)] {
            let lhs_rs = k + pad;
            let lhs: Vec<f32> = (0..m * lhs_rs).map(|_| next()).collect();
            let w: Vec<f32> = (0..n * k).map(|_| next()).collect();
            let got = matmul_rows_by_wt(&lhs, lhs_rs, m, &w, n, k);
            let want = reference(&lhs, lhs_rs, m, &w, n, k);
            for (g, r) in got.iter().zip(&want) {
                assert!((*g as f64 - r).abs() <= 1e-5 * (k as f64).sqrt().max(1.0), "{m}x{n}x{k}: {g} vs {r}");
            }
        }
    }
}
