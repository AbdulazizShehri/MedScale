//! OpenMed MLX weight layout (Spec 104).
//!
//! OpenMed's full-precision MLX exports (`*-mlx`, format `mlx-fp`) store the
//! Hugging Face token-classification weights unchanged in value and shape,
//! under renamed keys (`weights.safetensors`). The renaming is ported from
//! OpenMed v3.0.0 `openmed/mlx/convert.py` (`remap_key` and its
//! `_*_KEY_REPLACEMENTS` tables, commit `ea920f36`, Apache-2.0): ordered,
//! literal substring replacements from the Hugging Face key to the MLX key.
//! The candle runtime asks for Hugging Face names and this map tells it the
//! stored MLX name, so the same encoders run MLX exports on any platform.
//!
//! Quantized MLX exports (`mlx-8bit`, `mlx-4bit`) change tensor contents and
//! are not handled here.

const BERT: &[(&str, &str)] = &[
    (".attention.self.query.", ".attention.query_proj."),
    (".attention.self.key.", ".attention.key_proj."),
    (".attention.self.value.", ".attention.value_proj."),
    (".attention.output.dense.", ".attention.out_proj."),
    (".attention.output.LayerNorm.", ".ln1."),
    (".intermediate.dense.", ".linear1."),
    (".output.dense.", ".linear2."),
    (".output.LayerNorm.", ".ln2."),
    ("bert.encoder.layer.", "encoder.layers."),
    (
        "bert.embeddings.word_embeddings.",
        "embeddings.word_embeddings.",
    ),
    (
        "bert.embeddings.position_embeddings.",
        "embeddings.position_embeddings.",
    ),
    (
        "bert.embeddings.token_type_embeddings.",
        "embeddings.token_type_embeddings.",
    ),
    ("bert.embeddings.LayerNorm.", "embeddings.norm."),
    ("classifier.", "classifier."),
    ("bert.pooler.", "_pooler."),
];

const DEBERTA_V2: &[(&str, &str)] = &[
    (".attention.output.dense.", ".attention.out_proj."),
    (".attention.output.LayerNorm.", ".ln1."),
    (".intermediate.dense.", ".linear1."),
    (".output.dense.", ".linear2."),
    (".output.LayerNorm.", ".ln2."),
];

const ROBERTA: &[(&str, &str)] = &[
    ("roberta.encoder.layer.", "encoder.layers."),
    ("xlm_roberta.encoder.layer.", "encoder.layers."),
    (
        "roberta.embeddings.word_embeddings.",
        "embeddings.word_embeddings.",
    ),
    (
        "roberta.embeddings.position_embeddings.",
        "embeddings.position_embeddings.",
    ),
    (
        "roberta.embeddings.token_type_embeddings.",
        "embeddings.token_type_embeddings.",
    ),
    ("roberta.embeddings.LayerNorm.", "embeddings.norm."),
    (
        "xlm_roberta.embeddings.word_embeddings.",
        "embeddings.word_embeddings.",
    ),
    (
        "xlm_roberta.embeddings.position_embeddings.",
        "embeddings.position_embeddings.",
    ),
    (
        "xlm_roberta.embeddings.token_type_embeddings.",
        "embeddings.token_type_embeddings.",
    ),
    ("xlm_roberta.embeddings.LayerNorm.", "embeddings.norm."),
    ("roberta.pooler.", "_pooler."),
    ("xlm_roberta.pooler.", "_pooler."),
];

const DISTILBERT: &[(&str, &str)] = &[
    (".attention.q_lin.", ".attention.query_proj."),
    (".attention.k_lin.", ".attention.key_proj."),
    (".attention.v_lin.", ".attention.value_proj."),
    (".attention.out_lin.", ".attention.out_proj."),
    (".sa_layer_norm.", ".ln1."),
    (".output_layer_norm.", ".ln2."),
    (".ffn.lin1.", ".linear1."),
    (".ffn.lin2.", ".linear2."),
    ("distilbert.transformer.layer.", "encoder.layers."),
    (
        "distilbert.embeddings.word_embeddings.",
        "embeddings.word_embeddings.",
    ),
    (
        "distilbert.embeddings.position_embeddings.",
        "embeddings.position_embeddings.",
    ),
    ("distilbert.embeddings.LayerNorm.", "embeddings.norm."),
];

const MODERNBERT: &[(&str, &str)] = &[
    ("model.layers.", "model.encoder.layers."),
    (
        "model.embeddings.tok_embeddings.",
        "model.embeddings.word_embeddings.",
    ),
    (".attn.Wqkv.", ".attention.qkv_proj."),
    (".attn.Wo.", ".attention.out_proj."),
    (".mlp.Wi.", ".mlp.wi_proj."),
    (".mlp.Wo.", ".mlp.wo_proj."),
];

/// The stored MLX key for a Hugging Face key of `model_type`
/// (`remap_key` in OpenMed `convert.py`).
#[must_use]
pub fn mlx_key(hf_key: &str, model_type: &str) -> String {
    let tables: &[&[(&str, &str)]] = match model_type {
        "modernbert" => &[MODERNBERT],
        "deberta-v2" => &[DEBERTA_V2],
        "distilbert" => &[DISTILBERT],
        // OpenMed applies the RoBERTa table, then the BERT table.
        "roberta" | "xlm-roberta" => &[ROBERTA, BERT],
        _ => &[BERT],
    };
    let mut key = hf_key.to_owned();
    for table in tables {
        for (hf, mlx) in *table {
            key = key.replace(hf, mlx);
        }
    }
    key
}

/// True when a verified `config.json` describes an OpenMed MLX export.
#[must_use]
pub fn is_mlx_export(config: &serde_json::Value) -> bool {
    config["_mlx_weights_format"].is_string() || config["_mlx_model_type"].is_string()
}

/// True when the export is quantized (contents changed; not supported).
#[must_use]
pub fn is_quantized(config: &serde_json::Value) -> bool {
    !config["quantization"].is_null() || !config["quantization_config"].is_null()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_match_the_openmed_mlx_export() {
        // Keys observed in OpenMed-NER-AnatomyDetect-BioClinical-108M-mlx
        // (`46a92836`, weights.safetensors header).
        assert_eq!(
            mlx_key("bert.encoder.layer.0.attention.self.query.weight", "bert"),
            "encoder.layers.0.attention.query_proj.weight"
        );
        assert_eq!(
            mlx_key("bert.encoder.layer.3.attention.output.dense.bias", "bert"),
            "encoder.layers.3.attention.out_proj.bias"
        );
        assert_eq!(
            mlx_key("bert.embeddings.LayerNorm.weight", "bert"),
            "embeddings.norm.weight"
        );
        assert_eq!(
            mlx_key("bert.embeddings.word_embeddings.weight", "bert"),
            "embeddings.word_embeddings.weight"
        );
        assert_eq!(mlx_key("classifier.weight", "bert"), "classifier.weight");
        assert_eq!(
            mlx_key("bert.encoder.layer.0.output.LayerNorm.weight", "bert"),
            "encoder.layers.0.ln2.weight"
        );
        assert_eq!(
            mlx_key(
                "roberta.encoder.layer.1.intermediate.dense.weight",
                "xlm-roberta"
            ),
            "encoder.layers.1.linear1.weight"
        );
        assert_eq!(
            mlx_key(
                "distilbert.transformer.layer.0.attention.q_lin.weight",
                "distilbert"
            ),
            "encoder.layers.0.attention.query_proj.weight"
        );
        assert_eq!(
            mlx_key("model.layers.2.attn.Wqkv.weight", "modernbert"),
            "model.encoder.layers.2.attention.qkv_proj.weight"
        );
        assert_eq!(
            mlx_key(
                "deberta.encoder.layer.0.attention.output.dense.weight",
                "deberta-v2"
            ),
            "deberta.encoder.layer.0.attention.out_proj.weight"
        );
    }

    #[test]
    fn export_markers_and_quantization() {
        let mlx = serde_json::json!({"model_type": "bert", "_mlx_weights_format": "safetensors"});
        assert!(is_mlx_export(&mlx));
        assert!(!is_quantized(&mlx));
        assert!(!is_mlx_export(&serde_json::json!({"model_type": "bert"})));
        assert!(is_quantized(
            &serde_json::json!({"quantization": {"bits": 8, "group_size": 64}})
        ));
    }
}
