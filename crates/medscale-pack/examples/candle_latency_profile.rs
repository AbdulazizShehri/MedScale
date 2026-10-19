//! Warm forward-pass latency of the candle encoders MedScale runs, on
//! randomly initialised weights with a real model's config (latency does not
//! depend on weight values). Usage:
//!   cargo run --release -p medscale-pack --example candle_latency_profile -- <config.json> [seq...]
use candle_core::{DType, Device, Tensor};
use candle_nn::{VarBuilder, VarMap};
use candle_transformers::models::{bert, distilbert};
use std::time::Instant;

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args[0] == "layer" {
        layer_breakdown(args.get(1).map_or(16, |s| s.parse().unwrap()));
        return;
    }
    if args[0] == "unary" {
        let x = Tensor::randn(0f32, 1.0, (1, 16, 3072), &Device::Cpu).unwrap();
        let time = |name: &str, f: &dyn Fn() -> Tensor| {
            for _ in 0..5 {
                f();
            }
            let ms = median(
                (0..50)
                    .map(|_| {
                        let t = Instant::now();
                        f();
                        t.elapsed().as_secs_f64() * 1000.0
                    })
                    .collect(),
            );
            println!(
                "{name:10} {ms:.3} ms per 16x3072 ({:.1} ns/element)",
                ms * 1e6 / (16.0 * 3072.0)
            );
        };
        time("gelu_erf", &|| x.gelu_erf().unwrap());
        time("gelu_tanh", &|| x.gelu().unwrap());
        time("exp", &|| x.exp().unwrap());
        time("relu", &|| x.relu().unwrap());
        return;
    }
    if args[0] == "matmul" {
        // One BERT-base feed-forward projection: (1, seq, 768) x (3072, 768)^T.
        let device = Device::Cpu;
        let w = Tensor::randn(0f32, 1.0, (3072, 768), &device).unwrap();
        let wt = w.t().unwrap().contiguous().unwrap();
        for seq in [16usize, 64] {
            let x = Tensor::randn(0f32, 1.0, (1, seq, 768), &device).unwrap();
            let time = |f: &dyn Fn() -> Tensor| {
                for _ in 0..3 {
                    f();
                }
                median(
                    (0..25)
                        .map(|_| {
                            let t = Instant::now();
                            f();
                            t.elapsed().as_secs_f64() * 1000.0
                        })
                        .collect(),
                )
            };
            let linear = candle_nn::Linear::new(w.clone(), None);
            let a = time(&|| candle_nn::Module::forward(&linear, &x).unwrap());
            let b = time(&|| x.broadcast_matmul(&wt).unwrap());
            let gflop = 2.0 * seq as f64 * 768.0 * 3072.0 / 1e9;
            println!(
                "seq={seq:3} linear(w.t view)={a:.3} ms ({:.1} GFLOP/s)  contiguous w^T={b:.3} ms ({:.1} GFLOP/s)",
                gflop / a * 1000.0,
                gflop / b * 1000.0
            );
        }
        return;
    }
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&args[0]).expect("config")).expect("json");
    let seqs: Vec<usize> = if args.len() > 1 {
        args[1..].iter().map(|s| s.parse().unwrap()).collect()
    } else {
        vec![8, 16, 32, 48, 64, 128, 256]
    };
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let kind = config["model_type"].as_str().unwrap_or_default().to_owned();
    enum M {
        Bert(bert::BertModel),
        Distil(distilbert::DistilBertModel),
    }
    let model = match kind.as_str() {
        "bert" => M::Bert(
            bert::BertModel::load(
                vb.pp("bert"),
                &serde_json::from_value(config.clone()).unwrap(),
            )
            .unwrap(),
        ),
        "distilbert" => M::Distil(
            distilbert::DistilBertModel::load(
                vb.pp("distilbert"),
                &serde_json::from_value(config.clone()).unwrap(),
            )
            .unwrap(),
        ),
        other => panic!("unsupported model_type {other}"),
    };
    println!(
        "model_type={kind} threads(rayon)={} candle_num_threads={}",
        rayon_threads(),
        candle_core::utils::get_num_threads()
    );
    for &len in &seqs {
        let ids: Vec<u32> = (0..len as u32).map(|i| 1000 + i).collect();
        let input = Tensor::from_slice(&ids, (1, len), &device).unwrap();
        let types = Tensor::zeros((1, len), DType::U32, &device).unwrap();
        let ones = Tensor::ones((1, len), DType::U32, &device).unwrap();
        let none = Tensor::zeros((1, 1, 1, len), DType::U8, &device).unwrap();
        let run = || {
            let h = match &model {
                M::Bert(m) => m.forward(&input, &types, Some(&ones)).unwrap(),
                M::Distil(m) => m.forward(&input, &none).unwrap(),
            };
            h.flatten_all().unwrap().to_vec1::<f32>().unwrap().len()
        };
        for _ in 0..3 {
            run();
        }
        let samples: Vec<f64> = (0..15)
            .map(|_| {
                let t = Instant::now();
                run();
                t.elapsed().as_secs_f64() * 1000.0
            })
            .collect();
        println!("seq={len:4} median_ms={:.2}", median(samples));
    }
}

/// A BERT-base encoder layer written with the same candle ops as
/// candle-transformers' BertLayer, timing each op class over 12 layers.
fn layer_breakdown(seq: usize) {
    use std::collections::BTreeMap;
    let d = Device::Cpu;
    let (h, heads, ff) = (768usize, 12usize, 3072usize);
    let hd = h / heads;
    let r = |s: (usize, usize)| Tensor::randn(0f32, 0.02, s, &d).unwrap();
    let lin =
        |o, i| candle_nn::Linear::new(r((o, i)), Some(Tensor::zeros(o, DType::F32, &d).unwrap()));
    let (q, k, v, o) = (lin(h, h), lin(h, h), lin(h, h), lin(h, h));
    let (up, down) = (lin(ff, h), lin(h, ff));
    let ln = candle_nn::LayerNorm::new(
        Tensor::ones(h, DType::F32, &d).unwrap(),
        Tensor::zeros(h, DType::F32, &d).unwrap(),
        1e-12,
    );
    let mut x = Tensor::randn(0f32, 1.0, (1, seq, h), &d).unwrap();
    let mut acc: BTreeMap<&str, f64> = BTreeMap::new();
    let mut t = |name: &'static str, f: &mut dyn FnMut() -> Tensor| {
        let s = Instant::now();
        let out = f();
        *acc.entry(name).or_default() += s.elapsed().as_secs_f64() * 1000.0;
        out
    };
    use candle_nn::Module;
    // Includes two warm-up passes in the average (timing is relative).
    let iters = 12;
    for _ in 0..iters {
        for _layer in 0..12 {
            let split = |y: Tensor| {
                y.reshape((1, seq, heads, hd))
                    .unwrap()
                    .transpose(1, 2)
                    .unwrap()
                    .contiguous()
                    .unwrap()
            };
            let qh = t("linear qkv", &mut || q.forward(&x).unwrap());
            let kh = t("linear qkv", &mut || k.forward(&x).unwrap());
            let vh = t("linear qkv", &mut || v.forward(&x).unwrap());
            let (qh, kh, vh) = (
                t("reshape/transpose", &mut || split(qh.clone())),
                t("reshape/transpose", &mut || split(kh.clone())),
                t("reshape/transpose", &mut || split(vh.clone())),
            );
            let kt = t("reshape/transpose", &mut || kh.t().unwrap());
            let scores = t("attn matmul", &mut || qh.matmul(&kt).unwrap());
            let scores = t("scale+mask", &mut || {
                ((scores.clone() / 8.0).unwrap() + 0.0).unwrap()
            });
            let probs = t("softmax", &mut || {
                candle_nn::ops::softmax_last_dim(&scores).unwrap()
            });
            let ctx = t("attn matmul", &mut || probs.matmul(&vh).unwrap());
            let ctx = t("reshape/transpose", &mut || {
                ctx.transpose(1, 2)
                    .unwrap()
                    .contiguous()
                    .unwrap()
                    .reshape((1, seq, h))
                    .unwrap()
            });
            let a = t("linear out", &mut || o.forward(&ctx).unwrap());
            let a = t("add+layernorm", &mut || {
                ln.forward(&(a.clone() + &x).unwrap()).unwrap()
            });
            let f1 = t("linear ffn", &mut || up.forward(&a).unwrap());
            let f1 = t("gelu", &mut || f1.gelu_erf().unwrap());
            let f2 = t("linear ffn", &mut || down.forward(&f1).unwrap());
            x = t("add+layernorm", &mut || {
                ln.forward(&(f2.clone() + &a).unwrap()).unwrap()
            });
        }
    }
    let total: f64 = acc.values().sum::<f64>() / iters as f64;
    println!(
        "seq={seq} threads={} per-forward total {total:.2} ms (12 layers)",
        candle_core::utils::get_num_threads()
    );
    let mut rows: Vec<_> = acc.into_iter().collect();
    rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    for (name, ms) in rows {
        println!(
            "  {name:18} {:7.2} ms  {:5.1}%",
            ms / iters as f64,
            100.0 * ms / iters as f64 / total
        );
    }
}

fn rayon_threads() -> String {
    std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "default".into())
}
