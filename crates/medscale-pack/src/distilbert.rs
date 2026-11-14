//! DistilBERT encoder with Hugging Face activation semantics.
//!
//! Adapted from candle-transformers 0.9.1 `models/distilbert.rs`
//! (<https://github.com/huggingface/candle>, MIT OR Apache-2.0, copyright the
//! candle authors). The weight layout, attention and layer norms are
//! unchanged. One difference: candle maps the config activation `"gelu"` to
//! the tanh approximation, while Hugging Face Transformers maps `"gelu"` to
//! the exact erf GELU (`ACT2FN["gelu"]`, `GELUActivation`). This encoder
//! follows Transformers, so DistilBERT checkpoints compute what they were
//! trained with. Tracing spans and the masked-LM heads are omitted.
use candle_core::{DType, Result, Tensor};
use candle_nn::{Embedding, LayerNorm, Linear, Module, VarBuilder, embedding, layer_norm, linear};
use serde::Deserialize;

/// `config.json` `activation` values, as Hugging Face's `ACT2FN` names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Activation {
    /// Exact GELU, `0.5 x (1 + erf(x / sqrt 2))`.
    #[serde(rename = "gelu")]
    Gelu,
    /// The tanh approximation (`gelu_new`, `gelu_pytorch_tanh`).
    #[serde(rename = "gelu_new", alias = "gelu_pytorch_tanh")]
    GeluTanh,
    #[serde(rename = "relu")]
    Relu,
}

impl Activation {
    fn apply(self, xs: &Tensor) -> Result<Tensor> {
        match self {
            Self::Gelu => xs.gelu_erf(),
            Self::GeluTanh => xs.gelu(),
            Self::Relu => xs.relu(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Config {
    pub vocab_size: usize,
    pub dim: usize,
    pub n_layers: usize,
    pub n_heads: usize,
    pub hidden_dim: usize,
    pub activation: Activation,
    pub max_position_embeddings: usize,
}

struct Embeddings {
    word_embeddings: Embedding,
    position_embeddings: Embedding,
    layer_norm: LayerNorm,
}

impl Embeddings {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self> {
        Ok(Self {
            word_embeddings: embedding(config.vocab_size, config.dim, vb.pp("word_embeddings"))?,
            position_embeddings: embedding(
                config.max_position_embeddings,
                config.dim,
                vb.pp("position_embeddings"),
            )?,
            layer_norm: layer_norm(config.dim, 1e-12, vb.pp("LayerNorm"))?,
        })
    }

    fn forward(&self, input_ids: &Tensor) -> Result<Tensor> {
        let (_batch, seq_len) = input_ids.dims2()?;
        let positions = Tensor::arange(0u32, seq_len as u32, input_ids.device())?;
        let embeddings = self
            .word_embeddings
            .forward(input_ids)?
            .broadcast_add(&self.position_embeddings.forward(&positions)?)?;
        self.layer_norm.forward(&embeddings)
    }
}

struct SelfAttention {
    q_lin: Linear,
    k_lin: Linear,
    v_lin: Linear,
    out_lin: Linear,
    n_heads: usize,
    head_dim: usize,
}

impl SelfAttention {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self> {
        let dim = config.dim;
        Ok(Self {
            q_lin: linear(dim, dim, vb.pp("q_lin"))?,
            k_lin: linear(dim, dim, vb.pp("k_lin"))?,
            v_lin: linear(dim, dim, vb.pp("v_lin"))?,
            out_lin: linear(dim, dim, vb.pp("out_lin"))?,
            n_heads: config.n_heads,
            head_dim: dim / config.n_heads,
        })
    }

    /// `mask` is nonzero where attention is blocked, broadcastable to the scores.
    fn forward(&self, hidden: &Tensor, mask: &Tensor) -> Result<Tensor> {
        let (batch, len, _dim) = hidden.dims3()?;
        let heads = |t: Tensor| {
            t.reshape((batch, len, self.n_heads, self.head_dim))?
                .transpose(1, 2)
        };
        let q = (heads(self.q_lin.forward(hidden)?)? / (self.head_dim as f64).sqrt())?;
        let k = heads(self.k_lin.forward(hidden)?)?;
        let v = heads(self.v_lin.forward(hidden)?)?;
        let scores = q.matmul(&k.transpose(2, 3)?.contiguous()?)?;
        let blocked = mask.broadcast_as(scores.shape())?;
        let neg_inf =
            Tensor::new(f32::NEG_INFINITY, scores.device())?.broadcast_as(scores.shape())?;
        let scores = blocked.where_cond(&neg_inf, &scores.to_dtype(DType::F32)?)?;
        let weights = candle_nn::ops::softmax(&scores, candle_core::D::Minus1)?;
        let context = weights
            .matmul(&v.contiguous()?)?
            .transpose(1, 2)?
            .reshape((batch, len, self.n_heads * self.head_dim))?
            .contiguous()?;
        self.out_lin.forward(&context)
    }
}

struct Block {
    attention: SelfAttention,
    sa_layer_norm: LayerNorm,
    lin1: Linear,
    lin2: Linear,
    activation: Activation,
    output_layer_norm: LayerNorm,
}

impl Block {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self> {
        Ok(Self {
            attention: SelfAttention::load(vb.pp("attention"), config)?,
            sa_layer_norm: layer_norm(config.dim, 1e-12, vb.pp("sa_layer_norm"))?,
            lin1: linear(config.dim, config.hidden_dim, vb.pp("ffn.lin1"))?,
            lin2: linear(config.hidden_dim, config.dim, vb.pp("ffn.lin2"))?,
            activation: config.activation,
            output_layer_norm: layer_norm(config.dim, 1e-12, vb.pp("output_layer_norm"))?,
        })
    }

    fn forward(&self, hidden: &Tensor, mask: &Tensor) -> Result<Tensor> {
        let attended = self
            .attention
            .forward(hidden, mask)?
            .broadcast_add(hidden)?;
        let attended = self.sa_layer_norm.forward(&attended)?;
        let ffn = self
            .activation
            .apply(&self.lin1.forward(&attended)?)?
            .apply(&self.lin2)?;
        self.output_layer_norm.forward(&(ffn + attended)?)
    }
}

pub struct DistilBertModel {
    embeddings: Embeddings,
    layers: Vec<Block>,
}

impl DistilBertModel {
    /// Loads `embeddings.*` and `transformer.layer.N.*` under `vb`.
    pub fn load(vb: VarBuilder, config: &Config) -> Result<Self> {
        let layers = (0..config.n_layers)
            .map(|i| Block::load(vb.pp(format!("transformer.layer.{i}")), config))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            embeddings: Embeddings::load(vb.pp("embeddings"), config)?,
            layers,
        })
    }

    /// Hidden states for `input_ids` (batch, len). `mask` is nonzero where
    /// attention is blocked (shape broadcastable to batch, heads, len, len).
    pub fn forward(&self, input_ids: &Tensor, mask: &Tensor) -> Result<Tensor> {
        let mut hidden = self.embeddings.forward(input_ids)?;
        for layer in &self.layers {
            hidden = layer.forward(&hidden, mask)?;
        }
        Ok(hidden)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;
    use candle_nn::VarMap;
    use candle_transformers::models::distilbert as upstream;

    fn config(activation: &str) -> serde_json::Value {
        serde_json::json!({
            "model_type": "distilbert", "vocab_size": 50, "dim": 16, "n_layers": 2,
            "n_heads": 2, "hidden_dim": 32, "activation": activation,
            "max_position_embeddings": 32, "initializer_range": 0.02, "pad_token_id": 0,
            "dropout": 0.1, "attention_dropout": 0.1, "seq_classif_dropout": 0.2,
            "sinusoidal_pos_embds": false, "tie_weights_": true
        })
    }

    /// Both encoders over one set of random weights.
    fn outputs(activation: &str) -> (Vec<f32>, Vec<f32>) {
        let device = Device::Cpu;
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        let cfg = config(activation);
        let ours = DistilBertModel::load(
            vb.pp("distilbert"),
            &serde_json::from_value(cfg.clone()).unwrap(),
        )
        .unwrap();
        let theirs = upstream::DistilBertModel::load(
            vb.pp("distilbert"),
            &serde_json::from_value(cfg).unwrap(),
        )
        .unwrap();
        let ids = Tensor::new(&[[3u32, 7, 11, 2, 9, 4]], &device).unwrap();
        let mask = Tensor::zeros((1, 1, 1, 6), DType::U8, &device).unwrap();
        let flat = |t: Tensor| t.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        (
            flat(ours.forward(&ids, &mask).unwrap()),
            flat(theirs.forward(&ids, &mask).unwrap()),
        )
    }

    #[test]
    fn weight_layout_and_computation_match_candle_when_activations_agree() {
        // ReLU means the same thing in both: any difference would be layout.
        let (ours, theirs) = outputs("relu");
        let worst = ours
            .iter()
            .zip(&theirs)
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        assert!(worst < 1e-5, "max difference {worst}");
    }

    #[test]
    fn gelu_follows_hugging_face_exact_form_not_candle_tanh() {
        let (ours, theirs) = outputs("gelu");
        let worst = ours
            .iter()
            .zip(&theirs)
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        // Same weights, only the activation differs: small but real.
        assert!(worst > 0.0 && worst < 0.05, "max difference {worst}");
    }

    #[test]
    fn activation_names_follow_transformers() {
        let parse = |s: &str| serde_json::from_value::<Activation>(serde_json::json!(s)).unwrap();
        assert_eq!(parse("gelu"), Activation::Gelu);
        assert_eq!(parse("gelu_new"), Activation::GeluTanh);
        assert_eq!(parse("gelu_pytorch_tanh"), Activation::GeluTanh);
        assert_eq!(parse("relu"), Activation::Relu);
        assert!(serde_json::from_value::<Activation>(serde_json::json!("swish")).is_err());
    }
}
