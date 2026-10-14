//! Pre-download runtime compatibility for catalog rows (Spec 103, T103-09).
//!
//! This is an **expectation** derived from recorded execution evidence for the
//! local `tract_onnx_token_classification_v1` runtime
//! (`specs/103-local-model-catalog/research.md` sections 9–10). It is never a
//! status promotion: a row only becomes `RUNTIME_COMPATIBLE` / `EXECUTED_TESTED`
//! through its own evidence.

use serde::Serialize;

use crate::catalog::CatalogRow;

/// Architectures with at least one OpenMed v3.0.0 model executed end to end
/// (catalog → verified snapshot → signed Pack → admission → run). DeBERTa-v2
/// and XLM-R were added with qualification run 37977100903 (XLM-R through its
/// fp16 export, widened to fp32 at load).
pub const EVIDENCED_ARCHITECTURES: &[&str] = &[
    "bert",
    "distilbert",
    "roberta",
    "modernbert",
    "deberta-v2",
    "xlm-roberta",
    "eurobert",
];

/// One catalog row executed end to end on a real model: acquisition at the
/// catalog-pinned revision, verification, signed Pack, admission and run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExecutedModel {
    pub repo_id: &'static str,
    /// Hub commit the evidence ran against.
    pub revision: &'static str,
    /// `tract_onnx`, `candle_safetensors`, `candle_mlx` or `candle_gliner`.
    pub runtime: &'static str,
    pub weight_file: &'static str,
    /// GitHub Actions run id of the qualification evidence.
    pub evidence_run: &'static str,
}

/// Per-model execution evidence (rows, not architectures). Synthetic text
/// only: this records that the model runs, not task quality.
pub const EXECUTED_MODELS: &[ExecutedModel] = &[
    // Batch 26 (run 38490332415, 48 of 48 rows; PathologyDetect and PharmaDetect NER, per-row).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-PubMed-v2-109M",
        revision: "e00c20f6ecc89b2457ebdc345a31b18b553e67e4",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-MultiMed-335M",
        revision: "3b84752e2a1475e3851045b938ebc3d0b49a60a3",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-PubMed-335M-v1-onnx-android",
        revision: "8f84f908a2c9d191b691d87e0c85860aadafacec",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-PubMed-335M",
        revision: "47bcfc65afffbf64befb6f40134e5123ca919ff4",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ModernMed-149M-v1-onnx-android",
        revision: "6273a119d4d78ffcde8ef42876b49ded967028cc",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-PubMed-109M",
        revision: "2ca793682d222c231bcb450961b7cbc7714987ec",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ModernMed-395M",
        revision: "01d1b1c787a486964d35daf1d638bcb924450f6d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-PubMed-109M-mlx",
        revision: "c7b742af2bfbfe3776a35a8b879930b0bf52b311",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-MultiMed-335M-v1-onnx-android",
        revision: "1452ebbfeaaf864fc6260dee5fff7a7a8b71fac0",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-SuperClinical-141M",
        revision: "b6390d49728fcbb26efaa492f07e335df1eafebd",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ModernMed-395M-v1-onnx-android",
        revision: "bd8fcc382dd7b40ed67302902c3f9417ba9f3b9e",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-PubMed-109M-v1-onnx-android",
        revision: "cf2b0513f0f48b0406eaa2b8141fc9b4adade646",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-SuperClinical-141M-v1-onnx-android",
        revision: "f37a66c2c4f727c77586d1003d65b5c1d532a878",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-PubMed-v2-109M-onnx-android",
        revision: "dfb9c90722d6b2d2353406977fe9e1f7ab00f5c6",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-PubMed-v2-109M-mlx",
        revision: "489fd536690ac5f647dc2e22007c28f7be04026d",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-SuperClinical-141M-mlx",
        revision: "06797d7d26d875dd37e797ff9f3ec34646ed6be6",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-SuperClinical-184M-mlx",
        revision: "760a0044c8c5982e214e4560dba205d597f8e7d6",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-SuperClinical-184M",
        revision: "70caf52145cd281e1553f67d5c08a1ab8428a5f1",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-SuperClinical-434M-v1-onnx-android",
        revision: "5c0c60daf69b2ed6fa8121e5c9cc82d63be6e6b8",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-SuperClinical-184M-v1-onnx-android",
        revision: "c68fabc96e7b980654bd47e3cd0ed5c97c032b55",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-SuperClinical-434M",
        revision: "460b81ef65107d36df1fd34a4360359c121a09cc",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-SuperMedical-125M",
        revision: "83bd91122f300b6568301947feb24a6905580715",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-SuperMedical-125M-mlx",
        revision: "cf5f6ab9573bc9d223aa855c0bd0a8b13c057149",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-SuperMedical-355M",
        revision: "543589135644eb5ac3618f5829cb8f32b08bd7bc",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-SuperMedical-355M-v1-onnx-android",
        revision: "cd8bb2e960812ce56912b58c6e98c0a74272a20f",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-SuperMedical-125M-v1-onnx-android",
        revision: "f91eb0f2d978b58802661616ca34cf9814fd7353",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-TinyMed-135M",
        revision: "660c7acad93bf8c20a75861cfaee268cae24cd24",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-TinyMed-135M-v1-onnx-android",
        revision: "4586eeed24a87c9b7105a65381663313699a9399",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-TinyMed-135M-mlx",
        revision: "657f52f23b09f26d098907a8505fc06e7966d47f",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-TinyMed-65M",
        revision: "22ac82036743057ee3e43e5c42081372fc51a221",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-TinyMed-65M-mlx",
        revision: "ebb1c877c833e24873632664cc17fdd4236d004f",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-TinyMed-65M-v1-onnx-android",
        revision: "815f906d4f028d3ded90d5fde55b22223d51da63",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-TinyMed-66M",
        revision: "f16d09c1a8c3b604f3998a936f063930f05ea01e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-TinyMed-66M-mlx",
        revision: "24ee7898af315982f9d219170d8637d1b1ee0d33",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-TinyMed-82M",
        revision: "9aec6fbc41e84c1f4c5c157c973c695b50d58968",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-TinyMed-66M-v1-onnx-android",
        revision: "07f96ad6e98c73fe68212dc43742044071155b91",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-TinyMed-82M-mlx",
        revision: "259b4edf18372539d81c021ef7bd2dfb3a092908",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-TinyMed-82M-v1-onnx-android",
        revision: "bca8977cc6c1f2fedf01a2f0cc967bce0da791e2",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PharmaDetect-BioClinical-108M-mlx",
        revision: "a84415437fa40b1946eb47b3d2288e8d489224e7",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PharmaDetect-BioClinical-108M",
        revision: "c0a8d5eba13b7aa5291478799adbcef5f0ac2f1a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PharmaDetect-BioMed-109M",
        revision: "8e510d68011fe8937a10e015dc7439781ab0bdaa",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PharmaDetect-BioMed-109M-mlx",
        revision: "84fa217da5e1a9429daff4901530cb27b2a0b4d5",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PharmaDetect-BioClinical-108M-v1-onnx-android",
        revision: "63804fa09de178fddd5ab5082e24f221086c8394",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PharmaDetect-BioMed-335M-v1-onnx-android",
        revision: "f25c92b6ef743be40f8bccbce625a314938d3497",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PharmaDetect-BioMed-335M",
        revision: "e2b5518cba4f95154b1556947cf4660cee683b3b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PharmaDetect-BioMed-109M-v1-onnx-android",
        revision: "d7f86bfa6d556d5a9f8b1a5dbd9a44327b26ec95",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PharmaDetect-BioPatient-108M-mlx",
        revision: "93e7b248ae8de25d1d165c02f97c7ae7eec6bb19",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38490332415",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PharmaDetect-BioPatient-108M",
        revision: "c6a50beb895afa62fce8ced457a704f89b915a21",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38490332415",
    },
    // Batch 25 (run 38485176124, 48 of 48 rows; OrganismDetect and PathologyDetect NER, per-row).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-SuperClinical-184M-mlx",
        revision: "a3aaade1b195b134479529dfcb909a76e667e118",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-SuperClinical-434M-v1-onnx-android",
        revision: "af7ecfc46fe019132ce199db369cd5860a4ac483",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-TinyMed-65M",
        revision: "01497ce459a37f0335d19924c4e2a84b959c730c",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-SuperClinical-141M-v1-onnx-android",
        revision: "02ab823ef798e33785db8ebb851f7c902757a262",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-SuperMedical-125M-mlx",
        revision: "ac736d05a2849d156e9e7bc0a74cce861c628c0e",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-SuperClinical-184M",
        revision: "e86572ea155eb8e5ba95cee6813333c18afbc78d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-SuperMedical-355M",
        revision: "613e8986064203f4d70a8bb85d7e49ee64edb344",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-TinyMed-135M-mlx",
        revision: "fab9d01cfc0e040dce9f7a6c91d234d98dcb34ce",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-SuperMedical-125M-v1-onnx-android",
        revision: "7d9e2a312ee83d3f16695072646af83990cb829d",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-TinyMed-135M-v1-onnx-android",
        revision: "da460a7f9be577f7d201fff8b6605b76dc3cbeab",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-SuperMedical-355M-v1-onnx-android",
        revision: "ce390a48069379099de51c634add2e0ea376b24f",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-SuperMedical-125M",
        revision: "89ba8fbd5d2fb3170a8e51ab6b8fc63d9192f8ce",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-SuperClinical-184M-v1-onnx-android",
        revision: "b6409af576e3a06a79f2607c674c2cc825d5108f",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-TinyMed-65M-mlx",
        revision: "e54eb1e223fa48e6ce1f7378f3d21bccca11b7ff",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-TinyMed-135M",
        revision: "be9ccf41af667cb5302ceb623bca0fe37e9c309f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-SuperClinical-434M",
        revision: "f0155b52db981fafc5cebbe01d19e3011cd18b81",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-TinyMed-65M-v1-onnx-android",
        revision: "0004d23f2818e96a418c1fd59f208505f6f56ca7",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-TinyMed-66M",
        revision: "8af49f9e2164b82a89dc5ad7c7421be55b57193d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-TinyMed-82M",
        revision: "80b124106ffac6875e4040a9061dd8ecdfd20cce",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-TinyMed-66M-mlx",
        revision: "c3797aba2b31290e89b781b035b78e6db6c1200a",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-TinyMed-82M-mlx",
        revision: "196ee85407b1e43177cc35c3139dfd179889d391",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-TinyMed-66M-v1-onnx-android",
        revision: "be839e969bd9a5e38b2389f7ebdf27c1eb96f98b",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-BioClinical-108M",
        revision: "6f5c01db4c9d733dd85bddd6d0aa339a4574424d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-BioClinical-108M-mlx",
        revision: "4d4eead010cb1d38e22efb056550ddb546ecf76e",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-BioClinical-108M-v1-onnx-android",
        revision: "2abc851cdaf9d75b1a6d9dcfb04a2df0da51053a",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-TinyMed-82M-v1-onnx-android",
        revision: "f30527e5de3c1f2b633eb93fdb56e886204a405b",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-BioMed-109M",
        revision: "c336bcbf77d6ee192c9efcd4fc09f27837ec1f0a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-BioMed-109M-mlx",
        revision: "58cc3017333aeba2ab7fd9b760c0191307b10df2",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-BioMed-109M-v1-onnx-android",
        revision: "ccf3f39e82038d405ed9acdfae61595784a0eae9",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-BioMed-335M-v1-onnx-android",
        revision: "7ec66b864b7b48b19040bf8eccbcd1471a3ea4ba",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-BioMed-335M",
        revision: "a4caab078d1a8870d75cf63d84a5f8a0380362ab",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-BioPatient-108M",
        revision: "c8c0bd19993d665eb1620ff7023afe876b234519",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-BioPatient-108M-mlx",
        revision: "1c5f532d569f3b00858173e0397d3c50b71df885",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-BioPatient-108M-v1-onnx-android",
        revision: "2bb69f51fc00a313b851fa5b5b25bcff5328cdec",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ElectraMed-109M",
        revision: "f95aa51e0a36ca51ff71ebc80a0355b548652b63",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ElectraMed-109M-mlx",
        revision: "15a887b82bfe20c6ead67b13bec214003f08163f",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ElectraMed-109M-v1-onnx-android",
        revision: "b367af8a90a898e700fe0aef27037d230243c462",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ElectraMed-33M",
        revision: "a830c5871526e823c5fbaf3e1afb8c3f33b90ccf",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ElectraMed-335M",
        revision: "09fe3fbefea9787b2b673b586436d1e817864ed0",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ElectraMed-33M-mlx",
        revision: "086aa841793c47d9a819ab864a44ae9ac59e07eb",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ElectraMed-33M-v1-onnx-android",
        revision: "f6780f8dbe4becff5efdcf4803db80b1f6771633",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-EuroMed-212M-v1-onnx-android",
        revision: "fb6ed849df25df35ad97bc59e13b5fdcb4e0a5df",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ElectraMed-335M-v1-onnx-android",
        revision: "4b614c510a3acc3bf2e53e0451918ac0d99e778a",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ModernClinical-149M-v1-onnx-android",
        revision: "9871118733946263a04852d3fc1da6fd711cf133",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ModernClinical-149M",
        revision: "51dd33154bad322142268b2905bd02aa85baa4b2",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ModernClinical-395M",
        revision: "f591eba5f799c331e267b4c9cd1b129df740894f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ModernClinical-395M-v1-onnx-android",
        revision: "6c6bbccf72552c8f7a83cc7468ab0f61f3b42ea1",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38485176124",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-PathologyDetect-ModernMed-149M",
        revision: "264cd1b253cc6a761d7ee1b17414d91bc0565dca",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38485176124",
    },
    // Batch 24 (run 38479600818, 48 of 48 rows; OncologyDetect and OrganismDetect NER, per-row).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-BioClinical-108M",
        revision: "c0cf93a7ab63db7487aeac230241d203261f7a15",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-BioClinical-108M-v1-onnx-android",
        revision: "809b0a070752c607848362305d4b5705daacf943",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-BioMed-335M",
        revision: "b1c947157ccfec95b574612c52bee2e24453b8b4",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-TinyMed-82M-mlx",
        revision: "ad4163bc0e7a9538579405e0a2197a318517b194",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-TinyMed-65M",
        revision: "5dd4cdf7ec3200625ed87efcfb5f74699b1eb9b8",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-TinyMed-66M",
        revision: "fe1b18fb9d19412f3b30f02f2558e502839d6b9a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-TinyMed-65M-v1-onnx-android",
        revision: "8a1171bddc565f45bc2226e338756cef59d45f90",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-TinyMed-66M-mlx",
        revision: "8c2b272e73c676128ee5ffea0263dcaefc8711a3",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-BioMed-109M-v1-onnx-android",
        revision: "35ff666289b9a4e49be23d370d5f8bf5b73175e2",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-BioClinical-108M-mlx",
        revision: "28868ea64217fee11eb0ca9068feba893d3c5713",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-TinyMed-66M-v1-onnx-android",
        revision: "2435e3befe67d846cbc0fa08463adcacda12929c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-TinyMed-82M",
        revision: "b6188adf9aa3f1d450eb925fb67226dbc3e19be2",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-BioMed-109M",
        revision: "346674dd83f074c6d6c8c6525a92a0482bb11df7",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-TinyMed-65M-mlx",
        revision: "06e218c2c8a16e99f4fab21e9a6fe0a7e0106504",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-BioMed-109M-mlx",
        revision: "2b0cd8dea5f7ef85a8d2855dd8c7916f66b3a3a4",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-TinyMed-82M-v1-onnx-android",
        revision: "23618f6e58cd593276a196e3eaa7aafa90ab8a38",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-BioPatient-108M",
        revision: "bc18d6d72f8687d0a52290ef45f31d25ee471438",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-BioMed-335M-v1-onnx-android",
        revision: "c196f167315c3d0d47a3ef9672d23303dc9b1d34",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-BioPatient-108M-v1-onnx-android",
        revision: "24cb91e31951dc799a7a415aff4089b8cd49378b",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ElectraMed-109M",
        revision: "5f33449302579b953ee1b7c6d7eb6e4ca8f497ba",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-BioPatient-108M-mlx",
        revision: "805425c2f63ff3a3847aa570aff9944aae3402a4",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ElectraMed-335M",
        revision: "21215684eefea7f166f4ef483eb94b3716577e4a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ElectraMed-109M-v1-onnx-android",
        revision: "b409c50e9c9bdcbb431d9715321eb43b1051a908",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ElectraMed-109M-mlx",
        revision: "e5dc955266b17a5eeecf440c3d3a0006add0bc4b",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ElectraMed-335M-v1-onnx-android",
        revision: "e7164fd054274ae3966f5f4ede2f7dd79605e076",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ElectraMed-33M",
        revision: "2299c5d285040923328d63a16f3c292beb1ca338",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ModernClinical-149M-v1-onnx-android",
        revision: "e5c190f9c29f47599c96e14224b2812d6e72a3ad",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ElectraMed-33M-mlx",
        revision: "7373e4a3c6a85d9821f2434a0cd941401239e62d",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ElectraMed-33M-v1-onnx-android",
        revision: "51fd83264397d96ac0c1070ce7a853bf9d36cee9",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ModernClinical-149M",
        revision: "8a65f0957b75c5d4bba6f778969c7227bca1bbb5",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ModernClinical-395M",
        revision: "1d6c5bda8d8c61b778773d544090381f0c241d5a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ModernClinical-395M-v1-onnx-android",
        revision: "190235a4cf73899c2c1a77125de8d734b35ccdec",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ModernMed-149M",
        revision: "97988582ddc3c51fc0790ef0b3241487e1effc95",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ModernMed-149M-v1-onnx-android",
        revision: "fc59c80c67b42f8e0fe715901848306366ab9c1f",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ModernMed-395M",
        revision: "c05bebf9d63d41297fb32d734d8b44be67f82a5f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-ModernMed-395M-v1-onnx-android",
        revision: "18954deedd70253417a35d85745aab577d03b653",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-MultiMed-335M",
        revision: "bd7cf7ded3045b5e8db6e65e62a3283252728395",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-MultiMed-335M-v1-onnx-android",
        revision: "40c4006c910358b53d474c392768189fff7e43ee",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-PubMed-109M-mlx",
        revision: "7e3d8d290879403badf3f7fc5add5fcbcca7a6cf",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-PubMed-109M",
        revision: "1ddbdd569bc2e847c6d541165cc0cd2609ebe96b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-PubMed-109M-v1-onnx-android",
        revision: "de10cc9de8c3b4d92ba8f5588b98c2adde5e6bee",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-PubMed-335M",
        revision: "fb6a22b25bdcd076200d337bc2d53e6dd4af3b08",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-PubMed-v2-109M",
        revision: "c553f5f78042c5ff98f5f0b7832e048539a2e1a1",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-PubMed-335M-v1-onnx-android",
        revision: "05ed620fc8db23290209f6ab90194e31feecbe0e",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-PubMed-v2-109M-mlx",
        revision: "c86250c12bc9358fceb14aa060fe75cb06e0a0fd",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-PubMed-v2-109M-onnx-android",
        revision: "40e2fe7930e4ea09dc42b8f48fc702dd30546d06",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-SuperClinical-141M-mlx",
        revision: "5875b1ef5e4640ccffe53da18c71834d9cd4da27",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38479600818",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OrganismDetect-SuperClinical-141M",
        revision: "0472a9be18380df4aacb159f0899c5d97d0d056f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38479600818",
    },
    // Batch 23 (run 38473404618, 48 of 48 rows; GenomicDetect and OncologyDetect NER, per-row).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-SuperClinical-184M-v1-onnx-android",
        revision: "ba3a85ba42ddd747c468cb45ce9984c58d97386f",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-PubMed-v2-109M-mlx",
        revision: "9e21ba704a133035d35872aeaf05068666f30bfe",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-SuperClinical-141M-mlx",
        revision: "b8a4db92c0b962a6ae1dc9202813028653f5f792",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-SuperClinical-434M-v1-onnx-android",
        revision: "fe67625474f2009adc7f6886bc5daddbf88c9bfd",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-SuperMedical-125M-mlx",
        revision: "16cea11fd633cea82bdc97441323783cbd3b744b",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-ElectraMed-33M-v1-onnx-android",
        revision: "6c8d8193aa66307364ddc4aebebce65d3a29363c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-ElectraMed-335M",
        revision: "1d72a93c44e095ea40acc590f87a194f86cab671",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-ElectraMed-33M",
        revision: "2423d486e080f3e5f731f2e8a41191489fc57aa6",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-ElectraMed-33M-mlx",
        revision: "fd8f54a5daa1a16b8cbc02d81d6442631199db2e",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-SuperClinical-184M-mlx",
        revision: "f923aed94b063c9de61e8e6b06c6fba59f6de12e",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-SuperClinical-141M-v1-onnx-android",
        revision: "b60db1cf200863028992bd0f29c8fb8518421832",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-ElectraMed-335M-v1-onnx-android",
        revision: "f9b259cad44dffa054289bce9097a60486ce9c65",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-EuroMed-212M-v1-onnx-android",
        revision: "f2b7fd54e2a2fe24624b393a74020e0423782a91",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-PubMed-335M-v1-onnx-android",
        revision: "1635c16c3c2768159d2bb94df88486205aa5eb40",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-SuperClinical-434M",
        revision: "8a9a5ebc037d663679409edb03b873a86139b691",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-ElectraMed-109M-v1-onnx-android",
        revision: "05ebd998104fa9ff39f8fe4a48cdfbfde283f0be",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-ModernClinical-395M-v1-onnx-android",
        revision: "7efbb6705f02377bead8f4f5db88b83747affaeb",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-ModernClinical-395M",
        revision: "1d715ca88ecddeaea4f3bcdc36eace362c633b6e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-ModernMed-395M",
        revision: "48ca554d1e6677067a58114246a43be553e9e405",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-ModernClinical-149M-v1-onnx-android",
        revision: "69bfa8dbaf93b2ef0a1c66ac1b24ba08ab34c6b2",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-ModernClinical-149M",
        revision: "9d7c12496a53e22b327bdcc322bb38861a98628d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-MultiMed-335M",
        revision: "7380463b87ee48c9aa69c1ec9441eb991d83b7f4",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-ModernMed-395M-v1-onnx-android",
        revision: "4cec17b91c5e7cc316d8e788a8f9fe79f32ca248",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-PubMed-109M",
        revision: "3168a0fb54fe41461f297a9abe8be75ac68908c0",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-MultiMed-335M-v1-onnx-android",
        revision: "7c018f85692e8e74791cd25169f564200a46eb81",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-PubMed-109M-mlx",
        revision: "cec58cb10a078678b44b3d79e62d95b8ee9df91f",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-PubMed-335M",
        revision: "48eb6125c6a6a517ef7a804291a9cb97bc753adf",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-PubMed-335M-v1-onnx-android",
        revision: "1432a6d1692d963a71d85fb119dffc8b04fc97fb",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-PubMed-109M-v1-onnx-android",
        revision: "4386fb9cbc7e7cfb602a0814cbb0f65b595acf98",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-PubMed-v2-109M",
        revision: "9adffac4b05ca79584d8728178385022ac1e571a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-PubMed-v2-109M-mlx",
        revision: "93265ebabafb741934e0e3162f9e003c2985692b",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-PubMed-v2-109M-onnx-android",
        revision: "6bbf5d421851dfbb139498a8af6d9bb62665b430",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-SuperClinical-141M",
        revision: "90f4ce9783f73070e4daa6134d902af0d89912f2",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-SuperClinical-141M-mlx",
        revision: "493bc63c7de127a87d33bd799899041b845d5095",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-SuperClinical-184M",
        revision: "887c74e2d7279e0447408d23f509b6623abc1c38",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-SuperClinical-184M-mlx",
        revision: "fa4a6d3ce18525146a573bb218d9b3b37325be04",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-SuperClinical-434M",
        revision: "95364c370ca513e2f66f0d8f653451100a709f78",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-SuperClinical-141M-v1-onnx-android",
        revision: "142aea62e7c9eabf07ad865f4bb252ff192b6959",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-SuperClinical-184M-v1-onnx-android",
        revision: "f3ef0f5291cdc0fd6bf881f393a963e79b8a9cd2",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-SuperClinical-434M-v1-onnx-android",
        revision: "3d529610a8793f446f85faf828f64a4267a057d5",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-SuperMedical-125M-mlx",
        revision: "b4aae1365a46158d4d577b47bb00badeb45b6186",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-SuperMedical-125M",
        revision: "59149cd981a4ecd82bec7f022c2d1c65d83baf2f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-SuperMedical-125M-v1-onnx-android",
        revision: "c3b0fbef59dd1075a86469005295344127180e37",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-SuperMedical-355M-v1-onnx-android",
        revision: "233767e51810a2c652c57e07f7138bdd76ecaafa",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-SuperMedical-355M",
        revision: "4d897222dcc8b87959ab47a7bd438385edb86a0f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-TinyMed-135M",
        revision: "11bcf50e12d32fec0b9511b9eb6940e22620cdf6",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-TinyMed-135M-v1-onnx-android",
        revision: "c287a84fc832f87f263a75853602670a5719663c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38473404618",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-TinyMed-135M-mlx",
        revision: "9c510ec8b3bb477a31eaa1670d5949c49ce4de6c",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38473404618",
    },
    // Batch 22 (run 38466509604, 39 of 48 rows; GenomicDetect and OncologyDetect NER, per-row).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ModernMed-395M-v1-onnx-android",
        revision: "87066a1d3a8cebd82c4960f2afab597444451423",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-SuperClinical-141M",
        revision: "72d577a57f6a7766e2aac089bf92ff0fd106e9bb",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-PubMed-109M-mlx",
        revision: "d7e814e0b06c33230747d48ce87aae853687c3cf",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-PubMed-v2-109M",
        revision: "e77af8f617f929938528acd64ece1b02e0572bf5",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-PubMed-335M",
        revision: "b054c81ea07e6e2d0e6bda857ac9a9bc6500b34b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-PubMed-109M",
        revision: "9adcd4ff14e92b46ab44b3261787fb81307930b7",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-PubMed-v2-109M-onnx-android",
        revision: "b1802b07a69f6a2f81d4ae2fc05a112495d9095a",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-PubMed-109M-v1-onnx-android",
        revision: "985c86628b7252c16c61a65094b149fa8eb3b886",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ModernMed-395M",
        revision: "1e549b6d7cca158cfeb117e5af5c42696b61af91",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-MultiMed-335M-v1-onnx-android",
        revision: "416d77b2251c38e9fbb7f8a6d39140e135147db5",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-MultiMed-335M",
        revision: "9fba81ee7e0e85e66057071959d9b427158e5aa3",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-SuperMedical-125M",
        revision: "21a3f593a56ba630670a0c60c14f8af0498742d3",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-SuperMedical-355M-v1-onnx-android",
        revision: "ee1b3a3a42ab74d4f8877a6388520636f0096588",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-SuperMedical-355M",
        revision: "adbe4f376277ae554defb9be04d5347a140ab916",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-SuperMedical-125M-v1-onnx-android",
        revision: "03a43ab6414863ad1d7a106b907a5f5842849877",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-TinyMed-135M",
        revision: "1c8de464eb896d1935e4ce38ebd7bce97ee43ee3",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-TinyMed-135M-mlx",
        revision: "9d0030892789ac42b7f009e49a9f6bdc3ca1a790",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-TinyMed-135M-v1-onnx-android",
        revision: "0aa12841253a102e4018db336a1047bfaa05c301",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-TinyMed-65M-v1-onnx-android",
        revision: "25336b856f1974c4b8006f9d5ed92a047ee2527f",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-TinyMed-65M",
        revision: "4a4a6a9041f8757cc056347037ac669a61ec880c",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-TinyMed-66M-mlx",
        revision: "453b00e7d561780e87b7c15e1d3c0c13ed35cd80",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-TinyMed-66M-v1-onnx-android",
        revision: "15ef1880b3d5e29f15694b9dad80ad36dc54e519",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-TinyMed-65M-mlx",
        revision: "eadd0ecae793552bf5c88c1567d49d40bc2767e1",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-TinyMed-66M",
        revision: "9d2be7ff3703b4daadf18fe7605c553fee2a445b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-BioClinical-108M",
        revision: "71194df6ba759dccd457dae78da9a3f8d178886d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-TinyMed-82M",
        revision: "0b3ed0fd88c0953cfd1a4c5d06fb7fe77220bec6",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-TinyMed-82M-v1-onnx-android",
        revision: "835e274456ad0cea06baab6e090ac37e01790ec9",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-BioClinical-108M-mlx",
        revision: "763e53a54fd12b4a0b8a35f6f7d13f749cafd1d3",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-TinyMed-82M-mlx",
        revision: "c2ae013debe6492e1f18a2ab2c16942d7c369231",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-BioClinical-108M-v1-onnx-android",
        revision: "5d4de42e5e65f6d685401b2fcbf68add49994d41",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-BioMed-109M",
        revision: "f40874882e99f4966fc497b38c38bc74a77a075d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-BioMed-109M-mlx",
        revision: "510ac2e7fa97b07f329cf4f39a6d9bb882621e2f",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-BioMed-109M-v1-onnx-android",
        revision: "054132efcf602e6ae0cf2ce31b1892a27915401c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-BioMed-335M",
        revision: "f5c2d231a67b1d0ed368d5bc81cbac5907671e21",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-BioPatient-108M-mlx",
        revision: "86e0302a2e889a9e2526e9a5d34da9447c3f2ba7",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-BioMed-335M-v1-onnx-android",
        revision: "acb761c291c6aa2119ab3f4efa6601d7a2d3dcbc",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-BioPatient-108M-v1-onnx-android",
        revision: "b2fe857412244b3faa67b6357277118d4c730721",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-ElectraMed-109M",
        revision: "758d218ff0669263c38d951afe03ca297297e185",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38466509604",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-ElectraMed-109M-mlx",
        revision: "f98044ef06cdb781839085da31e5d4e8fced119c",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38466509604",
    },
    // Batch 21 (run 38459163759, 48 of 48 rows; GenomeDetect and GenomicDetect NER, per-row).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-SuperMedical-125M-v1-onnx-android",
        revision: "57b68bd3780d004da8401804f390252d4149f86b",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-BioMed-335M",
        revision: "5f397eba94e670bb5da392540df3bcbdc9feeec5",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-SuperMedical-355M-v1-onnx-android",
        revision: "25cd77a65758f4a1850885f71905530f5310d801",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-TinyMed-65M-mlx",
        revision: "141040ab2eaa2df80c662c660d3f4b92357cd28c",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-SuperMedical-355M",
        revision: "38f6bf856c55fb4d5093d734e9c94bfb671cd2dd",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-SuperMedical-125M-mlx",
        revision: "051a99a006ce73ba47c3b38e4224cd3c0afc14ad",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-SuperMedical-125M",
        revision: "99d1ab129e10e2d7b230ca78e7fcb4a3f707af33",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-SuperClinical-434M",
        revision: "28a9248f0cf3f071775baf0ead7937811038872b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-TinyMed-135M-mlx",
        revision: "53cba2415b400cbcee9cc9bf4cd717347e0c4cbb",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-BioMed-335M-v1-onnx-android",
        revision: "49a915817d74027d52e43e011e78553243c5de21",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-TinyMed-135M",
        revision: "fe70903b31b80080d307dc332af67bc82d0643bf",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-SuperClinical-434M-v1-onnx-android",
        revision: "bf6ba09528ddd9e2262e21692900d314b33e5fc7",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-BioMed-109M-v1-onnx-android",
        revision: "be0c63334064aecfcb8e2605f93cc65bc84c6d16",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-TinyMed-135M-v1-onnx-android",
        revision: "18c43d4e9a96f2d6039aab8e3bf4e122a947afc9",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-TinyMed-65M-v1-onnx-android",
        revision: "67d33ceb1ebf417ad41567c4100edcc55efa7573",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-TinyMed-65M",
        revision: "37f93201364677041212ec376f212813f2255baa",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-TinyMed-66M",
        revision: "632e70f0a0cd71760c4d236a08dd311602a5f988",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-TinyMed-66M-v1-onnx-android",
        revision: "40775d45466e614b8e603656fb002dc5d58f2cae",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-TinyMed-66M-mlx",
        revision: "31aa0668df77805d8e893dbf5cdeb9ae939123d1",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-TinyMed-82M",
        revision: "2268b27de642256a57e710b0011e45875160de87",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-BioClinical-108M",
        revision: "732515a6d16bebfc0e217ee05b7ec439379749c0",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-TinyMed-82M-mlx",
        revision: "3ad2b8d15a3c63b661ed65595032f8c108254941",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-TinyMed-82M-v1-onnx-android",
        revision: "c5e6b9a0f80228191fa042d5bba1778d76305521",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-BioClinical-108M-mlx",
        revision: "fc01ec8a2de69317eabf1478bbdbba72e332878d",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-BioMed-109M",
        revision: "2aa59063abf159200dbb8e3376d22092827f38a3",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-BioClinical-108M-v1-onnx-android",
        revision: "410f666aff897676c5c076a31e91d481f30fb0a7",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-BioMed-335M",
        revision: "c352b73de1b861008ece3189dfce2ecd40a162d0",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-BioMed-109M-mlx",
        revision: "2a64fa48cf974740eec849e29c3972a27a989ee3",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-BioMed-109M-v1-onnx-android",
        revision: "52254ace45776679136694e4d961e9585340c0ce",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-BioMed-335M-v1-onnx-android",
        revision: "101efa05f9d0072562a31faf8cd9b91267f48129",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-BioPatient-108M",
        revision: "41545f1effeae6c4dcbf59a203d4586377299f0d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-BioPatient-108M-mlx",
        revision: "9b1819c6c53aad1077d9664cd2a2641c5de1f0bd",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-BioPatient-108M-v1-onnx-android",
        revision: "5d27af293117070363a9b7c483ec64e785181e93",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ElectraMed-109M",
        revision: "be265ad79b5541e3af5d4bdda314822fcaccb884",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ElectraMed-109M-mlx",
        revision: "5932d8198e1b2cca77ef5b0971d0da90bbb18a36",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ElectraMed-109M-v1-onnx-android",
        revision: "92f1ccd1a09184902c8b438a4fad2a84542b90a2",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ElectraMed-335M",
        revision: "26eb784080f761d09af2836fb41354644e696ec3",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ElectraMed-335M-v1-onnx-android",
        revision: "1bca6924403c4c24cdc73031b320bc56cd4afcff",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ElectraMed-33M",
        revision: "552ea327d27d05655140426d13ca99d6805cb6c4",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ElectraMed-33M-v1-onnx-android",
        revision: "4369851bd876a384130d93806443bfa32827c3ff",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ElectraMed-33M-mlx",
        revision: "2b34c02edfcf6d4d491f8061367b40202673ed8e",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-EuroMed-212M-v1-onnx-android",
        revision: "79a2fca6c5709fe197071c7472dbca8c4aa045a8",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ModernClinical-149M",
        revision: "a850a905f3fbf187e0923d7618aa0f9ab50c3041",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ModernClinical-149M-v1-onnx-android",
        revision: "7273524a75dc85e846ad7e4d71329abc932af07e",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ModernClinical-395M",
        revision: "cb167a08c20fe9d0c0478116be5ab8e8fc6a9f60",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ModernClinical-395M-v1-onnx-android",
        revision: "42412b346d463dc15998b47217b2a4aff4ff9d10",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ModernMed-149M",
        revision: "af23856a6e31a8480bb4f9d52c652c0b8534d0aa",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38459163759",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-ModernMed-149M-v1-onnx-android",
        revision: "fabc1cecd05240df7f1858696839488d51ce639d",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38459163759",
    },
    // Batch 20 (run 38451112364, 45 of 48 rows; DiseaseDetect and GenomeDetect NER, per-row).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-TinyMed-66M-v1-onnx-android",
        revision: "640d2b13fc751e72112f3d55c996eb7c1d1dfef7",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-TinyMed-82M",
        revision: "207b1445e613611bc5e1165646d8e2788e3c8462",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-TinyMed-82M-mlx",
        revision: "195e448c17515957a2db8869b87da6211403357f",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-BioPatient-108M-v1-onnx-android",
        revision: "4fc55dfb2592d1dcd1df227048fa8d2795bc28e1",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-BioPatient-108M",
        revision: "66c94c46ef53dcc864f7392b48b6e896d2fdd566",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-BioClinical-108M-v1-onnx-android",
        revision: "d3b7d813d9fefe3464597363484eeb48673f168b",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-BioClinical-108M",
        revision: "f02631aa10ae3b2d908c0f07c129da185656785d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ElectraMed-109M",
        revision: "611a78e1025c4e4ba024e6b5a6cb1d69f80fbb6f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-BioPatient-108M-mlx",
        revision: "5436efe28bf527f967da17a997c2d2aff0cf82aa",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-TinyMed-82M-v1-onnx-android",
        revision: "45d0d7a3791bdfa8be7f9d08be10b207d5b5aeaf",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-BioMed-109M-mlx",
        revision: "0a89268bcd9ed68d0112dbaadf1b09e046375cbb",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-BioMed-109M",
        revision: "cd14587b9cfb25c7fda81fde0b59444aab30d60d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-BioClinical-108M-mlx",
        revision: "a9292fe5da3b836a482581c902dc2d8a0e2f6cce",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ElectraMed-109M-mlx",
        revision: "0db8ca871c8db9da9ad04263e0aeaa99624c1151",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ElectraMed-335M",
        revision: "8d9ef4410b9a452a6a8a089d7acd29654c243bd2",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ElectraMed-109M-v1-onnx-android",
        revision: "6bd6e6120979fde367964f8fa4b6960d3fadcfbc",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ElectraMed-33M",
        revision: "c5ef3dbb004cabdddf11089b91931f00585604d5",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ElectraMed-33M-v1-onnx-android",
        revision: "3934ffa01f1cbab421ea630088327f8075088f4b",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ModernClinical-149M",
        revision: "2b4d6c30926810bd6866367f48a6c8e8a5f446b8",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ModernClinical-149M-v1-onnx-android",
        revision: "4289ecc3a8d298cafed51535c98fd8cc1a2dfa01",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ElectraMed-335M-v1-onnx-android",
        revision: "c5ac41808a9b86544296cc34b5827577bcd8fb74",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ElectraMed-33M-mlx",
        revision: "7c88b1dc4c6e64d4391c4849df266f648df26055",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-EuroMed-212M-v1-onnx-android",
        revision: "889a78804cc5bea80f6a074a75206700f877bac8",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ModernClinical-395M",
        revision: "bc5c621dba86001d7974003f5bba1f54a9973ec3",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ModernClinical-395M-v1-onnx-android",
        revision: "d895db8f5c2830bd1d4719a60503bf6d947e9ec9",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ModernMed-149M",
        revision: "c5101d024117be31ba677a5fa0ee8bb1ec801a79",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ModernMed-149M-v1-onnx-android",
        revision: "4410e1641922374848c70b501e90535d6408e8bd",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ModernMed-395M",
        revision: "e1468fb47ee729a0e64c8d4c53161d6c1b2711bd",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-MultiMed-335M-v1-onnx-android",
        revision: "04c87d4efc78369c3bdcf8df2cce71242963cc28",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-MultiMed-335M",
        revision: "877efd6d8d784b56924384e074cbb4ce21d6d9d8",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-ModernMed-395M-v1-onnx-android",
        revision: "892ff21a96d2f36cf95a018c8fb36d7ed8bdbc3e",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-PubMed-109M",
        revision: "8d760f6bd3888ff6416a320992916a477c430e6e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-PubMed-109M-mlx",
        revision: "b65bdb3cd46f6bcd54dbc51407b0b7b1be83dd9a",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-PubMed-109M-v1-onnx-android",
        revision: "a27a1a0d08fa8e187695e33ffb319d93efb41cf0",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-PubMed-335M",
        revision: "a28a5ac26caee406bd83a9853cb5e3f3e7d70187",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-PubMed-335M-v1-onnx-android",
        revision: "77a11a5eb2b17b2a83d73aff6c6bae5ad373b2e0",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-PubMed-v2-109M",
        revision: "1f91cfa260db1f404e144ca0d1f6a3c3c82a995d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-PubMed-v2-109M-mlx",
        revision: "8fdedc4f56e549bd55cd3967283e7b89a81e94f3",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-PubMed-v2-109M-onnx-android",
        revision: "803bddb69e15c3acb7eeae2f03a12d4fef8fe6af",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-SuperClinical-141M",
        revision: "451b7e12f39418b72608bee6071196dda8f0add0",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-SuperClinical-141M-mlx",
        revision: "928337c226ee5cdc52aa793eaedf461bc87cfea7",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-SuperClinical-141M-v1-onnx-android",
        revision: "f936feb8baed2f2da36364ffb65514c93f6473ec",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-SuperClinical-184M",
        revision: "8bd25254dc183715e65debc981d8f26844eb5787",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-SuperClinical-184M-v1-onnx-android",
        revision: "b6a10bb43478917ed10604977dc6481409e588ec",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38451112364",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomeDetect-SuperClinical-184M-mlx",
        revision: "def22c2dd1240cfb59a082a0c80074454faeba93",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38451112364",
    },
    // Batch 19 (run 38443895041, 48 of 48 rows; DiseaseDetect NER, per-row).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-33M-mlx",
        revision: "20c10234c1ee4cbe02fe529189c8c4cb64dfe1f5",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-335M-v1-onnx-android",
        revision: "1533e59ed9c5e716859911927582f5ce1529677f",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ModernClinical-395M-v1-onnx-android",
        revision: "1aee5e9350ecf01afe0a8d279270354468c5776c",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ModernMed-395M",
        revision: "a9a8c40f00a4d5df866621ca7e76a8a07b4a6207",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-BioPatient-108M-v1-onnx-android",
        revision: "2f9bfa5c934b3a68143d76f83f3394f8d1f2d4f8",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-109M-mlx",
        revision: "811f16c0f23908bb3ff00f025effb6f62ef3ffde",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-109M-v1-onnx-android",
        revision: "709b531369cf8ab79702b34128e6a1c5f4338457",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ModernMed-149M-v1-onnx-android",
        revision: "916ce4447cedaf80561b34ac4356d08977869134",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ModernClinical-395M",
        revision: "b3c2c326ba796778076497927893bebc59ac420d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ModernMed-149M",
        revision: "043c02d1a66f8fb25b270772974383f1c96d3af1",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-BioPatient-108M-mlx",
        revision: "759efea03094f04a0cc3236695c2ba527ca2eed2",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ModernClinical-149M-v1-onnx-android",
        revision: "a26333c4661969035c2ed9a1052e0dbce0d01a99",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-335M",
        revision: "83c597b1095a2d880334f5cad6c65e2a6bfa0d55",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ModernClinical-149M",
        revision: "f6c96fabcd32a0c6afb7b9feb3ed589f17a34dd1",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-109M",
        revision: "15c3cd848c28757eef1c19e03ceaa2ecbadd2039",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-33M",
        revision: "05659fbdbb9088ede49bf52a35645fb37472db8b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ModernMed-395M-v1-onnx-android",
        revision: "3dc6f62cf2ca3111fa77986193cdfd9a09b8401a",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-PubMed-109M",
        revision: "253fde74a1a98eb40bb12d9e9ff5c8e92e2b146b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-MultiMed-335M-v1-onnx-android",
        revision: "c2d292785f977a7998c4663ef1f7be4ea4875d86",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-PubMed-109M-v1-onnx-android",
        revision: "c27ed616a0b2df118d98bec03e276b10eac6c915",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-MultiMed-335M",
        revision: "a694263d63acad555192273aa7500eed8719f1b3",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-PubMed-109M-mlx",
        revision: "a8e130274df0b714031b8000386586050cc9eb66",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-PubMed-335M",
        revision: "f75649007fa2e1f872b97dce628fba590e5608f7",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-PubMed-v2-109M",
        revision: "114b383bbc523168b6692ad0b4dc378ba0209014",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-PubMed-335M-v1-onnx-android",
        revision: "29147556496680a7190845ef2d9bc5148e1f4c95",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-PubMed-v2-109M-mlx",
        revision: "515403dd30fa790a2d9f1727a98da207758f2cc2",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-SuperClinical-141M",
        revision: "5e7fc5d7f39cc828f88dab9ab4dc90a731545915",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-PubMed-v2-109M-onnx-android",
        revision: "6e8fbd36175e93fc67da269b17177c870356eedd",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-SuperClinical-141M-mlx",
        revision: "7d255023bc51ad13f11d1ca6079146aac5fd2f4d",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-SuperClinical-141M-v1-onnx-android",
        revision: "2dae3469582bc16ceed60310d1c50be240f154a8",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-SuperClinical-184M",
        revision: "1d72b9a9bf3e755fef613337a7d6b17e1b7ef48f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-SuperClinical-184M-mlx",
        revision: "ef3919eaf8414ffd0a2bfaeb6cb53dd58aca8532",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-SuperClinical-434M",
        revision: "da6257021de179e8479c6b336f8bd274b525646f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-SuperClinical-434M-v1-onnx-android",
        revision: "ec584568b90e86ee03e94984f0933c17db68ea81",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-SuperClinical-184M-v1-onnx-android",
        revision: "f72c11ed31ec691ad97484faeb6ef2917c42a34a",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-SuperMedical-125M-mlx",
        revision: "f176fd22b2f29409f73fd0e021f22433369d3681",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-SuperMedical-125M",
        revision: "4bc0d813620c59e4ff6daa48e0b1a4732d021d14",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-SuperMedical-125M-v1-onnx-android",
        revision: "e5dbc19f0a465ae05ee309d2423998bd42f48a31",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-SuperMedical-355M",
        revision: "ef3d6a188c6e63af6eb4419d8c80617e839a2e45",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-SuperMedical-355M-v1-onnx-android",
        revision: "254694ebf5413b846197d8b5258688db43fc0e7e",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-TinyMed-135M",
        revision: "dcb346697f933cea9d9c9cb27b58a1faf8f08f5e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-TinyMed-135M-mlx",
        revision: "038c9223b7edecde0d48036cd84affa17660289f",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-TinyMed-65M",
        revision: "f61089545e3c9fc6f6bc93d1e6094ac0919a926e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-TinyMed-135M-v1-onnx-android",
        revision: "5d5de718e7cddc3f974eb92f54cf7a38bccd0050",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-TinyMed-65M-mlx",
        revision: "d920b3daa6572d64077c72b62cf7e471334128dc",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-TinyMed-65M-v1-onnx-android",
        revision: "7a73395c32aec2d7213476365dd45aa89f7c68b1",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-TinyMed-66M-mlx",
        revision: "b0adcc686e4a9629fd94a3fb8304698f375da793",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38443895041",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-TinyMed-66M",
        revision: "dc1b6d5ea2396225227f779f20f87b34ac35c96b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38443895041",
    },
    // Batch 18 (run 38434804059, 48 of 48 rows; DNADetect and DiseaseDetect NER, per-row).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ModernMed-149M-v1-onnx-android",
        revision: "59237ca6fa8da4f5f9726dcf10b4d1dde1e363be",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ModernMed-149M",
        revision: "5d4ab2accb5d215172fe65c35238dbbb3ec1fe28",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-PubMed-335M",
        revision: "a9bfc47a9a73217622aa6bc240db196936948b55",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-PubMed-335M-v1-onnx-android",
        revision: "31e6f08a8ff90c4e566841d45d9371ccf7b751bd",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-SuperClinical-141M",
        revision: "d43871d60c52fc42821bc7a423031d3d8c2600dd",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-PubMed-v2-109M-mlx",
        revision: "e1972c7a2e23eaa48e344be797739df6011ed892",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ModernMed-395M-v1-onnx-android",
        revision: "eb1098f3b7d5da9422eb459a6ec683a49dd92643",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-SuperClinical-141M-mlx",
        revision: "9774e2f5ed0122cd42611d675ce5d37b1888d8ad",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-MultiMed-335M",
        revision: "8cca5fb13963cf6e1287e2fae9ec856b54f5eca8",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-PubMed-109M-v1-onnx-android",
        revision: "42278abf5ad8d7d3050b00cc53f767c446a20169",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-PubMed-v2-109M",
        revision: "8d190bc38a8629e10b5fa526f3b3781fd9651919",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ModernMed-395M",
        revision: "9fed6e360cd0961392b9a14150fa380f2b9f519b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-MultiMed-335M-v1-onnx-android",
        revision: "e1ff50d2a565721caf7489c591c36f6058452bc0",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-PubMed-109M-mlx",
        revision: "05018a0e74b9fee3a55f4ac17ec63042fa35ca35",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-PubMed-v2-109M-onnx-android",
        revision: "523e37e16971c0086ee477f8535b4e261a617379",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-PubMed-109M",
        revision: "ee92bdc15abd69c271069379a167068751f18b65",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-SuperClinical-184M",
        revision: "78583647233f6aad6764c770950b4ffa352541fa",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-SuperClinical-434M-v1-onnx-android",
        revision: "cc118d11610dc7bf8e037a7d78a7ddf2b70280b5",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-SuperClinical-141M-v1-onnx-android",
        revision: "2fbc2910b8707e08a598e5d682df1ada6abcc174",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-SuperClinical-184M-v1-onnx-android",
        revision: "dc5c2efcc66fa099bc65b214a214d77a13c76fda",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-SuperMedical-125M",
        revision: "2c0f34199a82a1c49283491ffffe869c44116434",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-SuperClinical-434M",
        revision: "1ccc35aaedee9347cefae72584aed8349daf316b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-SuperClinical-184M-mlx",
        revision: "3cb3067942a89cff0918060803870a7b72b29ee4",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-SuperMedical-125M-mlx",
        revision: "ea9ab039a125efca9ca1f833f56f2f9751e6b3a5",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-SuperMedical-125M-v1-onnx-android",
        revision: "304398514e5510aeae9e42cb9e6f36f7b9d4589a",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-SuperMedical-355M",
        revision: "85eda0fad0624abcc65aceaeac160d0a0046500e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-TinyMed-135M-mlx",
        revision: "6542be5bb2f7832d3b01e53b7308b3b366256b31",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-TinyMed-65M",
        revision: "bbd45d6a587b92945b3acb093f73c7c56c444352",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-TinyMed-66M",
        revision: "09e12de21b3ab43bc0987355cef62bb3341e2924",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-TinyMed-65M-v1-onnx-android",
        revision: "ca91f5ae2ad7946a87b229ea77b3b1c761747cdd",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-TinyMed-135M",
        revision: "8e412f8616a9249a0eb6007ea3e7c3e3fafb48d4",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-TinyMed-66M-mlx",
        revision: "ae57e5413e4c4f1faf0fe3cf2015e1f37b6a66ad",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-SuperMedical-355M-v1-onnx-android",
        revision: "7528f1c99155247c1734c6bf60ccfcc939d69862",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-TinyMed-65M-mlx",
        revision: "fe85074cf713d3a72b0c2dd230174da493d808ba",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-TinyMed-135M-v1-onnx-android",
        revision: "c73efd4a24c94b158d02f7071aadaa17f7014ea9",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-TinyMed-66M-v1-onnx-android",
        revision: "b5f522d9019551e1b6648763571a54227cdf9db5",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-TinyMed-82M",
        revision: "9ced501db8ecc56f61470c66fcd54199dce5ce69",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-TinyMed-82M-mlx",
        revision: "1d9ee8cd3bd394e534715767c7e5e799cf7187b5",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-BioClinical-108M-mlx",
        revision: "3fc70b6f0bd5886af08f47d41d8b6fc672b86a27",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-BioClinical-108M",
        revision: "f25fd4b79677a7a9c16ed5450764753592c5aea1",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-TinyMed-82M-v1-onnx-android",
        revision: "dce5fb5d0f57ccf1fb9d523da011f6b56b4a9b03",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-BioClinical-108M-v1-onnx-android",
        revision: "b4ef0b1bb54e9cdbec1ea95d108229b81bba1e64",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-BioMed-109M",
        revision: "ae75341725e560f3a958b0fd6238d2931f4771d8",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-BioMed-109M-mlx",
        revision: "ae5ca73d37a6973b2d4b34918c2eb174140eb5c1",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-BioMed-335M",
        revision: "a62e2a2da5ff7d6b79024ba906f481581466ea61",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-BioMed-109M-v1-onnx-android",
        revision: "0aaa51ff799735511205d6279b05c31663372de9",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-BioPatient-108M",
        revision: "3c6d265ad07ed1c36baf855fb74ee9b36437f2ac",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38434804059",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-BioMed-335M-v1-onnx-android",
        revision: "f2b56204b837251789bfef81c5e6cf8d4360273b",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38434804059",
    },
    // Batch 17 (run 38393867391, 48 of 48 rows; BloodCancerDetect, ChemicalDetect and DNADetect NER, per-row).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-SuperClinical-184M",
        revision: "3f7af3df972d7c820db0d75a0a8a19328da56005",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-SuperMedical-125M",
        revision: "2a71497a85a051d9aca7a9a8b064ef951ffa47df",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-SuperMedical-355M-v1-onnx-android",
        revision: "c7fef21d584707be37e81652713965c988161d7a",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-SuperClinical-141M-v1-onnx-android",
        revision: "3aa4dc1545164f732ac55d5e0510d43fea0d8991",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-SuperClinical-141M-mlx",
        revision: "6074a39aa37ee9087eddf54e75455b64939059a7",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-SuperClinical-434M-v1-onnx-android",
        revision: "9ebdb9e794eb83a7da517ca120a49c171ba52175",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-SuperClinical-434M",
        revision: "eb08c2df5b40d67e523fa93dd6281a9af1971190",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-SuperClinical-184M-v1-onnx-android",
        revision: "4e8c746f722319bb87fc57d050387a2cb7204afd",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-SuperClinical-184M-mlx",
        revision: "58ac9e24053c11a7b97f7c15037980f3e5a4f262",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-TinyMed-135M-v1-onnx-android",
        revision: "36d5cc47e017d7e9876da2e27ab0394c821ea8f7",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-TinyMed-65M-mlx",
        revision: "0f2199fd98aa60648919209e5682ae05d8fc2099",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-TinyMed-135M",
        revision: "867ff7bdd5b35bfb844f817da639d92412d04178",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-SuperMedical-125M-v1-onnx-android",
        revision: "ef09d55bbba2bdfcf6f622754dda94b56ee835f5",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-TinyMed-135M-mlx",
        revision: "39915c8800dd2b37286a1c296e721998d8040977",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-TinyMed-65M",
        revision: "e31d33e202590337a2edb734e13c6969440ebfba",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-SuperMedical-125M-mlx",
        revision: "8c79851b81b3a3dd36d04658ce7b16a53b46e419",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-TinyMed-65M-v1-onnx-android",
        revision: "50b508f320a8d3b273afdbe4b35454f68e228aaa",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-TinyMed-65M-mlx",
        revision: "2a2f00bd8f146404dc475f77cdebeefd0f38d0b6",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-TinyMed-66M",
        revision: "202449de14b15bfe3e70b7ae726074400478090f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-TinyMed-66M-mlx",
        revision: "17e5c0a62d6aa7f0941b3abbe8459c7496bc7415",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-TinyMed-82M-mlx",
        revision: "fecf439829997f3184e35a4cb849fa0674658bf8",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-TinyMed-66M-v1-onnx-android",
        revision: "f3e68529e5d5b45454b7ad7f9727d8072a5e7419",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-TinyMed-82M",
        revision: "330271b8cf2081c10a0efcb7c3185df49d022767",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-BioClinical-108M",
        revision: "57f0c5c1b846e1b2548eca27bc2a54c1dceeb494",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-TinyMed-82M-v1-onnx-android",
        revision: "5e0bb3faa238e06e20d7cf57d86b1e877dde969c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-BioClinical-108M-mlx",
        revision: "6eb3f8db17c89ce7d7f5707f1ecbc606f176d9a8",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-BioClinical-108M-v1-onnx-android",
        revision: "9e0324c5d8241a9176fd0a2e81065acbdff3348d",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-BioMed-109M",
        revision: "af4663e245ef4fae9bf3f7d0e67a0882f7c87a9b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-BioMed-109M-mlx",
        revision: "e786b0f4bc3724bff4531e43ddcb2f0998e1f600",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-BioMed-335M",
        revision: "10a1cc987b966a9057aa4537fd8a220daf41f4a0",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-BioMed-335M-v1-onnx-android",
        revision: "7521014cffc7a147c172df104cd39da7f2d7c7c6",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-BioPatient-108M",
        revision: "81f7ada38beb6599e32ab8b89c18d602cc356bcc",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-BioPatient-108M-mlx",
        revision: "eb0ff17c8411baa1fab478e84bdf6ca99a8afb95",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-BioMed-109M-v1-onnx-android",
        revision: "7a8f012635e6b6fe77b17eddd0ece076c19eaacc",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-BioPatient-108M-v1-onnx-android",
        revision: "016a0268a89a59127b4a02bb8dfbd288bb2d3696",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ElectraMed-109M",
        revision: "2b66991298aacd8c1e6e1192726c738559e33773",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ElectraMed-109M-mlx",
        revision: "e69d8771bbde52d5e5767ff0354a00c6d6ab3e68",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ElectraMed-335M",
        revision: "7b4bafc93a3c33a0225162b67fe8af69f9108b07",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ElectraMed-109M-v1-onnx-android",
        revision: "9f761af75d743e466fcca910f3e35aeba4af3b97",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ElectraMed-335M-v1-onnx-android",
        revision: "e27c513009ec747e46aeba6d3ad53e1a0ac056f7",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ElectraMed-33M",
        revision: "bdab23bbe75e285f798af3cdfe2761aca4815d10",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ElectraMed-33M-v1-onnx-android",
        revision: "5889c3bcc4886b069ec3b9b62180e67890a259a4",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ElectraMed-33M-mlx",
        revision: "d87588eba9121d819da3a15958f314c0d8157f47",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-EuroMed-212M-v1-onnx-android",
        revision: "09265833c8bacb13308f3a346450de2bc5b4efa5",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ModernClinical-149M",
        revision: "9b6bd98db2026c2e78c6685c504e307b51b173ee",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ModernClinical-149M-v1-onnx-android",
        revision: "d4cef435bec253f3ecea8cfe5b088d1d298ae5e3",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ModernClinical-395M",
        revision: "22cd85369395a18088f467bd80ccb74d67b35406",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38393867391",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DNADetect-ModernClinical-395M-v1-onnx-android",
        revision: "3df4abf45ecf94333c135fcf8be26f454532b2e7",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38393867391",
    },
    // Batch 16 (run 38387956773, 47 of 48 rows; BloodCancerDetect and ChemicalDetect NER, per-row).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-TinyMed-135M-mlx",
        revision: "b15b2a3aa9c7a7e11b0a0511a8c1d43b8ac4f072",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-TinyMed-66M",
        revision: "b2a938e19173dc2290f23106dd12bea82c9d7b3d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-TinyMed-65M-v1-onnx-android",
        revision: "27ed38cd85998935c05f050e61e162eda29f5816",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-TinyMed-135M-v1-onnx-android",
        revision: "57426a5f8389eec882e3c2ef00ce3b073a651922",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-TinyMed-65M",
        revision: "62684ec5a6a211b91b7d5e33e7e6af95917b93ad",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-BioClinical-108M-v1-onnx-android",
        revision: "a83cf06f3ca1462a34e0c7b0c8460fec76331c69",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-TinyMed-82M",
        revision: "8398b988253b4aed4efbc6bd2dedb9a5b69dd0c8",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-BioMed-109M-mlx",
        revision: "607ab760320b8d7be65574695ba84776f66d8b62",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-TinyMed-82M-mlx",
        revision: "7543215ca61a8bf2a307f5f6a40f6a4e8a7b9c77",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-TinyMed-82M-v1-onnx-android",
        revision: "78e3b582d1d7426a92e53d075d50493d2c3551dd",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-TinyMed-66M-mlx",
        revision: "b7009831b7c7f3af83d9c499561bcd10998b8e83",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-BioClinical-108M",
        revision: "e891a568b358cb5487e43a855d36c79a1c19af0d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-BioClinical-108M-mlx",
        revision: "85accc7f190cec629aa026ce494a8d69a8e23258",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-BioMed-109M",
        revision: "c7393ecb1bd58fa0d82d519114ca54c12ad60567",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-TinyMed-66M-v1-onnx-android",
        revision: "0273e570a887441adf32693d5cef32e33c2f61db",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-BioMed-335M",
        revision: "784a9ec441aad6e65af980b2724d6efeae10b06b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-BioMed-335M-v1-onnx-android",
        revision: "1d244f247a0d30a0825cbbb9036b8f7b707e9aca",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-BioMed-109M-v1-onnx-android",
        revision: "f54f5ebec5d6b800a5e01a3f6e760b83bce6a5cb",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-BioPatient-108M",
        revision: "881b1574aac42543a9f5919f72d40567945fe93b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-BioPatient-108M-mlx",
        revision: "443f067165b58795bba3454d0d35a187749d8d64",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-BioPatient-108M-v1-onnx-android",
        revision: "1b6c7f88053f43e0218f457778c3b05d194eaf05",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-ElectraMed-109M",
        revision: "90cbac7f43d0c16e9b4df2a177e451eee1a3dff5",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-ElectraMed-109M-v1-onnx-android",
        revision: "cb9cd64214291648955cdc91f3f9c4345ac3ab41",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-ElectraMed-109M-mlx",
        revision: "6dd2f1e72907941549504e9eb3b7f7580b96461c",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-ElectraMed-33M",
        revision: "51549a709f9d961215bec3eb9369d134cae60989",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-ElectraMed-33M-mlx",
        revision: "1420d8f467aafe0acb44f2faad2272e00f6c8724",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-EuroMed-212M-v1-onnx-android",
        revision: "6c7bd4f10f10200f18b4392b3b9619a4378466cc",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-ModernClinical-149M",
        revision: "ede2413cf32d81862a427f29f99d6a8f37d04637",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-ElectraMed-33M-v1-onnx-android",
        revision: "567532d97c3fc7ba70eaad6ff589e8b1f7948e2f",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-ModernClinical-149M-v1-onnx-android",
        revision: "a9022a9af11bb9c5e7792e59ad9485fe04497160",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-ModernClinical-395M",
        revision: "92c13363c1a2b5bd0efd9f6c19e243d5488ef47d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-ModernMed-149M",
        revision: "bdf26940068efbc184adf861ddf2ffdea15c6a87",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-ModernMed-149M-v1-onnx-android",
        revision: "8b8fbfd7f9b6dc61c1ee0c5336fbb9453fe733c7",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-ModernClinical-395M-v1-onnx-android",
        revision: "476cacf6775546f0c40809cb3cb974a8b26d00f8",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-ModernMed-395M",
        revision: "44731cb969caa7f5c1eacbe748ff2e246db12e0a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-MultiMed-335M",
        revision: "c715c50692582e986997b782be5b67a167ab62b4",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-ModernMed-395M-v1-onnx-android",
        revision: "2322f8e81140f6502c51abc0618361b119eabde6",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-MultiMed-335M-v1-onnx-android",
        revision: "4216081f12191fda1519a28078bde99b93058975",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-PubMed-109M",
        revision: "78f35d7364748515e098a95a857d4c88c38902ae",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-PubMed-109M-mlx",
        revision: "25ed6495b7db5c11395eed74f8337213854ef6a9",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-PubMed-109M-v1-onnx-android",
        revision: "64e0d91860efe8567058f28b5093596f04e0b1ff",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-PubMed-335M",
        revision: "8e959d07d09d6229a84116c632c80da87db48843",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-PubMed-335M-v1-onnx-android",
        revision: "a2746f5b3d8a91698c916f1f088cd4af80ab5324",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-PubMed-v2-109M",
        revision: "467b63346447333e138a950431cfb17face5faf1",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-PubMed-v2-109M-mlx",
        revision: "7a14c457ed2bf0873207cfa430c0c2d2ccef39af",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-PubMed-v2-109M-onnx-android",
        revision: "ee69133286ab40ffd7c358c06d21025bb7268e1c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38387956773",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-SuperClinical-141M",
        revision: "6c0645eed0bc11249e9a43f2f7b4e509e9ab7987",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38387956773",
    },
    // Batch 15 (run 38381786129, 48 of 48 rows; BloodCancerDetect NER, per-row).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-BioMed-335M-v1-onnx-android",
        revision: "41c4f8a3cbe4993e7e610e87e930b282fe9fa6e5",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-BioPatient-108M",
        revision: "59490f8e61df872d2859d04eefdc7fd9ed4d8ee2",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-BioMed-335M",
        revision: "3f58b375cdaa798737dfdc1b2aecfb1b88f41aaa",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ElectraMed-109M",
        revision: "013fd418775a4e8b6e28f1ba3205ea63a16603fa",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ElectraMed-335M",
        revision: "3c629c0c3aeb44cebb2bfee1c96efa7425eec513",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-BioMed-109M-v1-onnx-android",
        revision: "45831a044259c285db618047069afe0e6aa6c207",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ElectraMed-33M",
        revision: "0bff905135b54f75d8b1a809969e87af519b6747",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ElectraMed-109M-v1-onnx-android",
        revision: "eb3e4b15f0d1c75a6b39b3518afb941afec1efb5",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-BioPatient-108M-v1-onnx-android",
        revision: "34fb0103a82a95f15f75fa4d1ba4d3522d8ac43c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ElectraMed-109M-mlx",
        revision: "a91c95878cc4479b85715bc6dcc4b2d124ef796b",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-BioPatient-108M-mlx",
        revision: "aecd2a5c71e0ba58459275f2749aa8b59374ada9",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-BioMed-109M",
        revision: "b0a857460c539937927fd94e60ee676123075be5",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-BioMed-109M-mlx",
        revision: "3afaced91da1ed81f38ed0a46de5878a1d0876d0",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ElectraMed-33M-v1-onnx-android",
        revision: "83ba7fb7eac2686969f37c71efe47a41294149e1",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ElectraMed-33M-mlx",
        revision: "9c8105e78004839d5beaf82d758a9dfc887e6e24",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ElectraMed-335M-v1-onnx-android",
        revision: "fff535422be51cabaeb4b969b5b557809bc3e239",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ModernClinical-149M",
        revision: "ddca214d2d3335df42600e76edcc4c05cd7485bc",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ModernClinical-395M",
        revision: "20d5b88fe0b6f4e7da71b89d0ae4071b0abf0bdb",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ModernClinical-149M-v1-onnx-android",
        revision: "2402a398f9eb73528475a9e3b6895037607950ec",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ModernMed-149M",
        revision: "e10a3d5f9e4e777e221523cfde70c329eafdcb06",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ModernMed-395M",
        revision: "37cadfa352445bf591422230717b8be46adfc60a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ModernClinical-395M-v1-onnx-android",
        revision: "0d5653b412efa949d254cce378af3ce2bd324dba",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-MultiMed-335M",
        revision: "8de088bf9d1a3624085cf2e970ccde8e39c876aa",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ModernMed-149M-v1-onnx-android",
        revision: "692b52f3a3e4097194bfaa7c2b727a267d978d28",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-ModernMed-395M-v1-onnx-android",
        revision: "886b64a97918c70659db68b3841e2cc3b6761cec",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-MultiMed-335M-v1-onnx-android",
        revision: "f154127b1aeb20f5f2f95b8980a7c38d09bc79b1",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-PubMed-109M",
        revision: "f6776d4214b78efbd0281418f81ef5798121801a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-PubMed-109M-v1-onnx-android",
        revision: "4e185edaf92e2fe91ddd31daa5ccda2bc6079709",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-PubMed-335M",
        revision: "68d5b65be48770aa93ccc9ecef26a9def6def30f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-PubMed-109M-mlx",
        revision: "e34131998c26f913ba814d0b53364cd9fe81c904",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-PubMed-335M-v1-onnx-android",
        revision: "4d0fbbb40e122015ae8eec346133fa4ce33d754b",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-PubMed-v2-109M",
        revision: "9f29aed7e232fa881e7b6a741d0f0ac4f2dfde3b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-PubMed-v2-109M-mlx",
        revision: "afb6fe5e0f9847612f3a6c988ef9109253e5f756",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-PubMed-v2-109M-onnx-android",
        revision: "3a10bb07fe8e6c94e5d90a62721cc1fd87e316e9",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-SuperClinical-141M-mlx",
        revision: "1ec4f942daaeb07bf98472bac5aeff7989e4434e",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-SuperClinical-141M",
        revision: "58e1c02505cf644c2158309b156227ffa4ee4481",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-SuperClinical-141M-v1-onnx-android",
        revision: "7786bd98fb60693949b7a83cadd8390ef4368f13",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-SuperClinical-184M-mlx",
        revision: "154d494e165a5e5ece0c52532005d1fa1fea4f62",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-SuperClinical-184M",
        revision: "6f204b78ef7bf8894b62ea266d86239f2087a4f7",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-SuperClinical-434M-v1-onnx-android",
        revision: "13471c8e7e12839b391177941a6cb2d083086ad9",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-SuperClinical-434M",
        revision: "d82d860f03243dfa4474ad179941ea36652aa6ed",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-SuperClinical-184M-v1-onnx-android",
        revision: "4e81cbf000961fcf6a0e4c75ffb365d3cf9c3f3f",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-SuperMedical-125M",
        revision: "d27286d45183c20c3d3f1259fc1759d62d400262",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-SuperMedical-125M-v1-onnx-android",
        revision: "10da808f01a81943a323af7aae6b7b9b59a61c28",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-SuperMedical-125M-mlx",
        revision: "e3921c65beadcaecb4ac304216bfdd1428e71ac0",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-SuperMedical-355M",
        revision: "bd4a702c746d18602305dc5ef141c04639a221f0",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-TinyMed-135M",
        revision: "04cbd4595239ee1871db8003be681dfde6f3e380",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38381786129",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-SuperMedical-355M-v1-onnx-android",
        revision: "ad881c4c6d0a7f6e09556e813166b6869b7f40ee",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38381786129",
    },
    // Batch 14 (run 38374107804, 48 of 48 rows; NER rows, first per-row batch).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-109M-v1-onnx-android",
        revision: "b07b5b41cfeb55d6184f4ab8dbc0c38598be05c2",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-BioClinical-108M",
        revision: "f6445838fc6e0e384e99eb9ee2ad54dd3da25609",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-BioPatient-108M",
        revision: "db03184911596a30b631ceb33d2009da940b8e29",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-335M-v1-onnx-android",
        revision: "edddd939cc5a3d1d6c1c3a70a2bcd23d79a3beb6",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-335M",
        revision: "4a2c038dd8ebc9aa8b40fd4a95e8b7fa79fe4382",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-BioClinical-108M-mlx",
        revision: "87357630b0e5b70f107f4d060fd5f5501039fb9c",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-BioMed-109M-mlx",
        revision: "add1a2f955fd5f83365181953b6f8c8680711786",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-33M-v1-onnx-android",
        revision: "1736f530c7348dda306604ac4d3abed3d74d79ed",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-BioClinical-108M-v1-onnx-android",
        revision: "28d9aa7801e2fbcfe67204f7798b0df82c65fb9c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-BioMed-335M",
        revision: "a227a43cac5094171cc3bfb150e8ea66e958a47a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-109M",
        revision: "207f22237c9b992750b45fbc4003066c50c4fbe1",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-109M-mlx",
        revision: "b4f14991686104d0d5c63d04be0d16a49fc3dc77",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-BioMed-335M-v1-onnx-android",
        revision: "0f944bdc4bf582f3eaccca2704b128dc66e6e671",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-BioPatient-108M-mlx",
        revision: "a20addf2f0121e0127cbb992a0b49aad78f6a64b",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-BioMed-109M-v1-onnx-android",
        revision: "8dcd96a67220bce88ddd1ea16e045ab7464c0cff",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-BioPatient-108M-v1-onnx-android",
        revision: "c196a62830945229754bb69cb6b84cfd5c164a30",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ModernClinical-395M",
        revision: "4e6a1564e77d8b8736f0b9d83a0f3d3a43bf68b3",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ModernClinical-395M-v1-onnx-android",
        revision: "bd97934917df5b794af6d9f058923b54f4961317",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ModernMed-149M",
        revision: "147280de3d867544aa2233ee759aeaf3888e9cb4",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ModernMed-395M-v1-onnx-android",
        revision: "2433e278211f123e6d97c62b8a7d42ea297df93d",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ModernMed-395M",
        revision: "f4c607a49c6d47e3153bbb9e9d5a1b334624d99e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ModernMed-149M-v1-onnx-android",
        revision: "bdee3fa0026f4e3d91f3f2e7fecf4fb05dc53482",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-MultiMed-335M",
        revision: "76483bbe52e0f14944a58f1c52816008f4673a40",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-MultiMed-335M-v1-onnx-android",
        revision: "eccb8a0391d6103c0663cb642a6fcc2fc7996023",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-PubMed-109M",
        revision: "10d8d3a7f6bd2667bafb3afce75f27525f53b2fe",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-PubMed-109M-mlx",
        revision: "d9b46d7489edcb41d716f824e889fc9581de4610",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-PubMed-109M-v1-onnx-android",
        revision: "9892c64325a38f9794b3b12c5c32ad64a3c97a47",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-PubMed-335M",
        revision: "34e40f30c8b1985ffb019bbc32cfe80cd2af1652",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-PubMed-335M-v1-onnx-android",
        revision: "a397460852128737a2a40f16f8d9bb55475d424d",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-PubMed-v2-109M-mlx",
        revision: "948c10f3190af4b549d21d36f1a49e481e6db670",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-PubMed-v2-109M",
        revision: "f06bf613c2071d86f1336d75b359d0dd13f394eb",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-PubMed-v2-109M-onnx-android",
        revision: "df0fda8a39cf974284288715b6fe54257cced3bc",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-SuperClinical-184M-mlx",
        revision: "b576ce4ddb23c76f7b13a64a806c0a817d7f3575",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-SuperClinical-184M",
        revision: "7c5a943c44fdc951a12cd9067db88f932a1e58ed",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-SuperClinical-184M-v1-onnx-android",
        revision: "a6c21ef7562b4b1b7d62fef4a91e30d837a4ebad",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-SuperClinical-434M",
        revision: "8e175ed62aebfb3d418781cd5768e5ba1357400f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-SuperClinical-434M-v1-onnx-android",
        revision: "6597a66af810eadba27c3b0fbf1ebb9570f6e26c",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-SuperMedical-355M",
        revision: "1714c49e1749300d04efc6475a9d927206779e36",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-SuperMedical-355M-v1-onnx-android",
        revision: "0abdfada880fef51719cfd454a006f87fd47e404",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-135M",
        revision: "e26dc4bb65b131850099a18cfa5f0a26cbac5d87",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-135M-mlx",
        revision: "bca0808c006a2e3ca2ab9af3dd924eaf6a71246c",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-135M-v1-onnx-android",
        revision: "f7cd37c8840c190f25df0218abec6a76729dec4f",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-66M-mlx",
        revision: "b961705c76596355767a04e24a1a15f851971f01",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-66M",
        revision: "bdb016bf9402ff88d03a6106fe2c93e430791f4e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-66M-v1-onnx-android",
        revision: "3094f6b3d2db62fc9ba838d547a461bd06938267",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-BioClinical-108M",
        revision: "0d54681164b4ed5902e721266068a3a24efd36c1",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-BioClinical-108M-mlx",
        revision: "d3421da255f0fd659c6db2cf2a926a29d6e803e8",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38374107804",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-BloodCancerDetect-BioClinical-108M-v1-onnx-android",
        revision: "d286a249015d8f6e17afb04f22815004b2d82fe5",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38374107804",
    },
    // Batch 13 (run 38373925933, 10 of 10 rows; Vietnamese PII and remaining groups).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-mLiteClinical-Base-135M-v1",
        revision: "3f12cf0e68b81b02072050197c4a58c978859321",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38373925933",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Vietnamese-SuperClinical-Base-184M-v1",
        revision: "7960bf5842c314e7258b5873a02e7708443ed061",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38373925933",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Vietnamese-SuperMedical-Large-355M-v1-onnx-android",
        revision: "021b5f323b38c8e1ab9315e869358d89c6d6c6b6",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38373925933",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Vietnamese-SuperClinical-Base-184M-v1-mlx",
        revision: "42bc5dde40eb47457a5f82943312357ba48071f8",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38373925933",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-mLiteClinical-Base-135M-v1-onnx-android",
        revision: "9bb6c30bd6fdab600dbfc00aa29a946391993077",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38373925933",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Vietnamese-SuperClinical-Large-434M-v1-onnx-android",
        revision: "4f8370e154dd542c54095c8e39c07b1d9da3597f",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38373925933",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Vietnamese-SuperClinical-Small-44M-v1-onnx-android",
        revision: "665e766d817110a648d9cc38d074041702ca9675",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38373925933",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Vietnamese-SuperClinical-Small-44M-v1-mlx",
        revision: "3dc1f01cf53d055f8568add0bd7462658998a104",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38373925933",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Vietnamese-SuperClinical-Base-184M-v1-onnx-android",
        revision: "4c8b167a7e7a64cd710ae6ac13ce534c6571cdf9",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38373925933",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Vietnamese-SuperClinical-Small-44M-v1",
        revision: "f59723b61cb9141242524c65d227ba4132547d2c",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38373925933",
    },
    // Batch 12 (run 38368604917, 24 of 24 rows; Turkish and Vietnamese PII).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-ClinicalLongformer-Base-149M-v1",
        revision: "6a5b1fea6f705c3488e446b1c4022e197d793c8e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-SuperClinical-Base-184M-v1",
        revision: "dd896f5ec6c6d9ad73a5b43346289848b0618a43",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-ClinicalE5-Small-33M-v1-onnx-android",
        revision: "df4b9c32779b50779dde82dba742070ec4f70629",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-LiteClinical-Small-66M-v1-onnx-android",
        revision: "a509348932892d6f75f41ec1151f888fb8be625b",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-SuperClinical-Small-44M-v1",
        revision: "a90ddf22c72ffab48d4ceb1833e5b99c0f292cee",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-FastClinical-Small-82M-v1-onnx-android",
        revision: "af55f17a95c6edc5b20c8a04728afec4c3db4f71",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-SuperClinical-Large-434M-v1-onnx-android",
        revision: "5e83ee402db32ab5888f82828d455bb634dec921",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-LiteClinical-Small-66M-v1-mlx",
        revision: "2a957d79049d435131555d5e544c69d23c49ecae",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-FastClinical-Small-82M-v1-mlx",
        revision: "c940a235ee2b682f1230e0f5be1e581d20f0b879",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-SuperClinical-Small-44M-v1-mlx",
        revision: "5fa1eaccf14615c31996c6608f6692eff4cdd653",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-LiteClinical-Small-66M-v1",
        revision: "f24054413c24141c26fc8f059f5b7f20e05dcbc7",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-SuperClinical-Base-184M-v1-onnx-android",
        revision: "83f7ceec4f0a112e41298b1a0bd587b1fc32c93f",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-FastClinical-Small-82M-v1",
        revision: "2deada8a5e56c0e5d78f14c7eeb40bf1d7a0d38f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-SuperClinical-Base-184M-v1-mlx",
        revision: "c80441a7e2d489abf5d85d10e8da4e64501ab384",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-SuperMedical-Base-125M-v1-mlx",
        revision: "3e150dc59c0921f08131fef6925695d1a0d59431",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-SuperMedical-Base-125M-v1-onnx-android",
        revision: "35f7e64338d040f362bbb695a95d2c4451b2f6f3",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-SuperMedical-Large-355M-v1-onnx-android",
        revision: "ef26c9d295376165386fefaf7a3075906b34f122",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-SuperMedical-Base-125M-v1",
        revision: "b7039a4f0b15608848a96a2d490afff52f9cff00",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-SuperClinical-Small-44M-v1-onnx-android",
        revision: "2a5b2b9675b84a871eb9003f0377af1fa57d1c73",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-mLiteClinical-Base-135M-v1-mlx",
        revision: "84593ee17451e4b4efb758f7472d384a11be2c9a",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Vietnamese-BigMed-Large-278M-v1-onnx-android",
        revision: "2c996ac92e24c939755a8197c3f574c4f65874cf",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-mLiteClinical-Base-135M-v1-onnx-android",
        revision: "2de5d19d4986626315e8d9d2da8688f778ada5d4",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-mLiteClinical-Base-135M-v1",
        revision: "aa9ae4a3d473879ab7abc4a0d27eaf0cc4ad9304",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38368604917",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Vietnamese-ClinicalBGE-Large-335M-v1-onnx-android",
        revision: "624a08fac7199bf55210035810c292469c1408a8",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38368604917",
    },
    // Batch 11 (run 38363574699, 24 of 24 rows; Telugu and Turkish PII).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-LiteClinical-Small-66M-v1",
        revision: "89882a30b4dde14f0471c4a7463a5b1f42c99e3a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-SuperClinical-Small-44M-v1",
        revision: "aa4f05a0fe39bca7eab17218dcee5c00cb424278",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-SuperMedical-Large-355M-v1-onnx-android",
        revision: "d3227b4f7faac3b14c29fcd21179b6595514fcad",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-EuroMed-Large-210M-v1-onnx-android",
        revision: "c3ee6649c951fee6d6d8b59bb462b5ffb13d0674",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-mLiteClinical-Base-135M-v1-onnx-android",
        revision: "e4fd1fa456d85cd9eecf57943e7d10bcb9f7b013",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-SuperClinical-Large-434M-v1-onnx-android",
        revision: "579916c5c5f9cee045926a7c80c1ff23b3ca739d",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-FastClinical-Small-82M-v1-onnx-android",
        revision: "16f041829baf62d3eb71689ee944dd7633b4471d",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-mLiteClinical-Base-135M-v1",
        revision: "ece40477b9cfef10405d1efe50d05f9a15ac8b41",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-SuperClinical-Base-184M-v1",
        revision: "68e0e1512ac2b46abd3c0e024e152436a2aeef44",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-SuperMedical-Base-125M-v1-onnx-android",
        revision: "751d1ccbcecdaffc1fc41154dee4e320027054a2",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-LiteClinical-Small-66M-v1-onnx-android",
        revision: "3f716fa5863c399b053db8b0e3cbab2765543443",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-SuperClinical-Base-184M-v1-onnx-android",
        revision: "89a471f95a3df185119fa564f6031d8b832d2099",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-SuperClinical-Small-44M-v1-onnx-android",
        revision: "892ae44206c2cd16da718a2fd8a2b01c5619e9a9",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-SuperMedical-Base-125M-v1",
        revision: "9c7ca925849d4de3c9f529c0877b01130d01d40e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-BioClinicalBERT-Base-110M-v1",
        revision: "b44b51798a10f6a1afeb08b4e7534ded5384b61a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-BioClinicalBERT-Base-110M-v1-mlx",
        revision: "14df55882e0a96ae26c4790d70ee4c937919d7fe",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-BigMed-Large-278M-v1-onnx-android",
        revision: "ae031b739d027216e3fcbb7855391f79708311d4",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-BioClinicalModern-Base-149M-v1",
        revision: "694d8eb81bddacb1c333241e0c867b31ebf44202",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-BioClinicalBERT-Base-110M-v1-onnx-android",
        revision: "51131592e54c34d7db41981ab2f536241071c9cc",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-BioClinicalModern-Base-149M-v1-onnx-android",
        revision: "033556a36a082179d310da905afee86d80cfe78c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-BioClinicalModern-Large-395M-v1-onnx-android",
        revision: "942935353560829e02d5cb21cda8faa98574a694",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-ClinicalE5-Small-33M-v1-mlx",
        revision: "8c7191f9b8a7f861a8af852322476021b923e077",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-BiomedBERT-Large-340M-v1-onnx-android",
        revision: "7867d48647fb7cf4975e7b72864f3f01ca70e53a",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38363574699",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-ClinicalE5-Small-33M-v1",
        revision: "380d44b95070648446c4a532259a02992dc5e006",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38363574699",
    },
    // Batch 10 (run 38358974839, 22 of 24 rows; Spanish, English and Telugu PII).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-mLiteClinical-Base-135M-v1-onnx-android",
        revision: "13e8006ed781b75cedde04e02fa3a96fbab983c2",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-SuperMedical-Large-355M-v1-onnx-android",
        revision: "361d92b89d9248e867b21e742c6d3a4910d4436f",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-BioClinicalBERT-Base-110M-v1-onnx-android",
        revision: "a766940d179e332aace47fa643c44aa94f5a721d",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-SuperClinical-Base-184M-v1-onnx-android",
        revision: "d3f8ba80fd81c8e5556c8a860d613b2f261cc8d8",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-BigMed-Large-278M-v1-onnx-android",
        revision: "30c452da14275ee442beeff71a57ada45fe767d4",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-mLiteClinical-Base-135M-v1",
        revision: "4e74267cd5146d1af082dde428fc72494f705a42",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-SuperMedical-Base-125M-v1",
        revision: "52647364fe8f553e2a901809e7ecbe9f96243523",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-SuperMedical-Large-355M-v1-onnx-android",
        revision: "237b210996a182a77ec1a57ea5753075573b6c70",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-SuperClinical-Base-184M-v1",
        revision: "11c6dfe63fc891c30787e1859e78b81aeb0d4cca",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-SuperClinical-Large-434M-v1-onnx-android",
        revision: "456c51d78313dacd38557ed049ea938f22ec16ad",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-SuperClinical-Small-44M-v1",
        revision: "a2360d3f42526fc660ac3b2b2301e1c2d94eba61",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-SuperClinical-Small-44M-v1-onnx-android",
        revision: "82f57fcab68125b05f1aa9fdd41319732358311b",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-SuperMedical-Base-125M-v1-onnx-android",
        revision: "28f8c4b64f8e8191499b58828e8640c6fc61ab4a",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-BioClinicalBERT-Base-110M-v1",
        revision: "774e707bb4215bfd0538c2855b61678edacc6dab",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-BioClinicalModern-Base-149M-v1",
        revision: "0d92108e7fed3754fc475f5223a7306047a700c9",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-BiomedBERT-Large-340M-v1-onnx-android",
        revision: "f63f70bf818aaad2eca6cb16755b68ba2d8e295b",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-BioClinicalModern-Large-395M-v1-onnx-android",
        revision: "53e90d1a64c28f39c31681f5cd1c4374c40a1d91",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-BioClinicalModern-Base-149M-v1-onnx-android",
        revision: "ba7e030f5ea80874384d4315cb213c044a8ac966",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-ClinicalE5-Small-33M-v1-onnx-android",
        revision: "f6120d32b2db38c7482f5e5a2c1cad2803e0d27b",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-ClinicalE5-Small-33M-v1",
        revision: "547e8c58bb0542a3b3ecbff1bb1c6b96ede4e44c",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-ClinicalLongformer-Base-149M-v1",
        revision: "00612ab6dec3f1339ffef66a9ff071c5fd30580f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38358974839",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-FastClinical-Small-82M-v1",
        revision: "15bd333ad0eb8abae22c1bca737a8a97c2aedd36",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38358974839",
    },
    // Batch 09 (run 38354857673, 24 of 24 rows; Portuguese and Spanish PII).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-BioClinicalBERT-Base-110M-v1",
        revision: "4ec9e7ecdbb2ea510fc95a59a770cba6b09573d3",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-ClinicalLongformer-Base-149M-v1",
        revision: "0e85d0c35e9acd9e7b05603c3ab0354ad9d94b58",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-mLiteClinical-Base-135M-v1",
        revision: "ee0a56f5857cf9dfa2bfe80ef5dff02f3e74626d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-BioClinicalModern-Base-149M-v1-onnx-android",
        revision: "71f9730bd4e8f5891ea00b9a4e49ae45f0951272",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-BioClinicalBERT-Base-110M-v1-onnx-android",
        revision: "bd16dfefc9ad0e10e20f11a98229bb8575530cfa",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-mLiteClinical-Base-135M-v1-mlx",
        revision: "b671bcb2ece31d8abd7c18dab1060fc197894e96",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-BioClinicalModern-Base-149M-v1",
        revision: "c0e535b81cd92c0011500379052935f7c0fc3f21",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-BioClinicalModern-Large-395M-v1-onnx-android",
        revision: "4fe297bcb9067d583195a430c61841093f9105b5",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-mLiteClinical-Base-135M-v1-onnx-android",
        revision: "f8f28b3f1569b81f7c907618b97189ab22d09220",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-BiomedBERT-Large-340M-v1-onnx-android",
        revision: "c4dd668c3f0900812f83660ed3563484e89f130a",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-FastClinical-Small-82M-v1",
        revision: "ba24397841780f65ae480a74f750c2b509762d05",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-ClinicalE5-Small-33M-v1-onnx-android",
        revision: "0d0332e706a15791d5906127a96147c6f3874182",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-ClinicalE5-Small-33M-v1",
        revision: "82d8692df8811b6f79ba510aa0e49ef3ebbf3592",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-EuroMed-Large-210M-v1-onnx-android",
        revision: "ec1daddd33a7e6f58b35a77e956814305631c781",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-LiteClinical-Small-66M-v1",
        revision: "55ecc0b362a5e11803785f0d38dc70da09a9195e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-LiteClinical-Small-66M-v1-onnx-android",
        revision: "39f3315e4677e4e2ae748b156dfc3a4605a59bf0",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-FastClinical-Small-82M-v1-onnx-android",
        revision: "89071ea767ab812ef6e8cc11a0ee8d841e1d6379",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-SuperClinical-Large-434M-v1-onnx-android",
        revision: "1281b747c33fbdcdf142225e742efc3dbba16e3e",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-SuperClinical-Base-184M-v1",
        revision: "b5c39baf5db4206a945a737920aa12840abd48d6",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-SuperClinical-Base-184M-v1-onnx-android",
        revision: "4354153e69403444829b5f11e296ac92cffe539e",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-SuperClinical-Small-44M-v1",
        revision: "9fb17321731076514a4cfc7ffbc1d31e5b64eda5",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-SuperMedical-Base-125M-v1",
        revision: "2a903c4b9d83eccd21ae1908958bc0219630bc25",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-SuperClinical-Small-44M-v1-onnx-android",
        revision: "fbeafaea46d918c55a8146bf76f577e77d27e7f3",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38354857673",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-SuperMedical-Base-125M-v1-onnx-android",
        revision: "a4e1d5fd59ad9260582fd7f2a9fee7c746680f3b",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38354857673",
    },
    // Batch 08 (run 38350788210, 24 of 24 rows; Portuguese PII).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-LiteClinical-Small-66M-v1-mlx",
        revision: "4cf8809c599312131ef9a5c6b337716b2fb3470a",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-ClinicalE5-Small-33M-v1-onnx-android",
        revision: "069952057a3613a7962f561239c2d0841da21944",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-LiteClinical-Small-66M-v1",
        revision: "e37c55aa229fa1131195ef1e6245a04a9afb8e5d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-ClinicalE5-Small-33M-v1-mlx",
        revision: "d4786398f0efdcb6177be594e8befdc1478a0e13",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-BiomedBERT-Large-340M-v1-onnx-android",
        revision: "98ff95df8d7b8951355b008170edd2b874829331",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-ClinicalLongformer-Base-149M-v1",
        revision: "d1745e7d52f93dda70e8245ad2068a5011b431d6",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-FastClinical-Small-82M-v1-mlx",
        revision: "e7a899a9a703ef50f08fa9e22d2baa5a52b8b0fd",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-ClinicalE5-Small-33M-v1",
        revision: "01a1677dea6daab4cb31d60a4ec3b1176a4a0244",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-LiteClinical-Small-66M-v1-onnx-android",
        revision: "d774b32f1cec404d8da9a8ff8ad0b5ab67c2f9e2",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-SuperClinical-Base-184M-v1",
        revision: "99a2e9ca098d04a416b3357c5945f395fca66796",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-FastClinical-Small-82M-v1",
        revision: "a6243988757f2236827a8989be4aa532d1d2aba1",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-BioClinicalModern-Large-395M-v1-onnx-android",
        revision: "ca2761eae331a84f9a82cb5265db8037809ce074",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-FastClinical-Small-82M-v1-onnx-android",
        revision: "88d930b025d507fd960e850b3413ad301925e840",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-BioClinicalModern-Base-149M-v1-onnx-android",
        revision: "e6289a361347caad5b4dc0c0ecfec1536316078a",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-SuperClinical-Base-184M-v1-onnx-android",
        revision: "a124aa9f1a310d5b60637e6616a03d947a5b2f7e",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-SuperClinical-Large-434M-v1-onnx-android",
        revision: "c84a1a21952bc19d4783345ccb1912af6abfeef6",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-SuperClinical-Base-184M-v1-mlx",
        revision: "78a83a82f8132ac235f59581d49ac8b225214ffd",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-SuperClinical-Small-44M-v1-mlx",
        revision: "994f1f06690247b719d7e0dfd534819270e5a907",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-SuperClinical-Small-44M-v1-onnx-android",
        revision: "1ae9008ef7caa7a63417effeddbc7d16fa70435a",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-SuperClinical-Small-44M-v1",
        revision: "e74186eb0bd7800ea68d9ee9442e8621cb2a4928",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-SuperMedical-Base-125M-v1",
        revision: "c021f98822beb191e3ab825a50bf09f12e9881f8",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-SuperMedical-Base-125M-v1-mlx",
        revision: "793441fc80ea452f4d65c1e3e7f2feb0330f3236",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-SuperMedical-Base-125M-v1-onnx-android",
        revision: "4582e59022c6e6023f582503acde7f64b85adaa3",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38350788210",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-SuperMedical-Large-355M-v1-onnx-android",
        revision: "4b7a1cc61de8bfd88d6343216e917a803103eff4",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38350788210",
    },
    // Batch 07 (run 38347129440, 24 of 24 rows; Italian, Portuguese,
    // Japanese and Korean PII).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-SuperClinical-Base-184M-v1-onnx-android",
        revision: "5d5f09f49d8caee7abe878e2d7edda56bc8577e0",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-FastClinical-Small-82M-v1-onnx-android",
        revision: "05cef784e0234e82c5806d2ec2451a1385c1d962",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-LiteClinical-Small-66M-v1-onnx-android",
        revision: "ec892ebbc8f961a70f0332a7ee726a9dd6ccf92f",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-ClinicalLongformer-Base-149M-v1",
        revision: "2c664f6b974502da9853ac4eae996d9d212562e9",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-FastClinical-Small-82M-v1",
        revision: "60e841c60a2d17d51558a1a1a65806c3aef123b0",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-LiteClinical-Small-66M-v1",
        revision: "69f8c5c13963befa3096f9fb1567a599d3133afe",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-SuperMedical-Base-125M-v1",
        revision: "7129b53a07da36d79f72878d4ed031c76e71f90a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-SuperClinical-Small-44M-v1",
        revision: "6b47b973d3590dacffe920a02ab50247f4a8a134",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-EuroMed-Large-210M-v1-onnx-android",
        revision: "a93f28b5a150bc6c3dc983da83c46791c6d72ea4",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-SuperClinical-Base-184M-v1",
        revision: "e0a1a41711b9e58acac2e4d1a42665cd14de8dae",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-SuperMedical-Base-125M-v1-onnx-android",
        revision: "f2b7b81a325301cd883dc0ebfce5dbd22626146d",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-mLiteClinical-Base-135M-v1",
        revision: "0ce2c464154c6d5c7fa7ccd58ece0949cebee178",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-SuperClinical-Small-44M-v1-onnx-android",
        revision: "200be8fec86f167645a9f46ee80e7c87fa772c0f",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-SuperMedical-Large-355M-v1-onnx-android",
        revision: "f528e4a02c144cad26aa1144f2a952f09ae52eb3",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Japanese-NomicMed-Large-395M-v1-onnx-android",
        revision: "d243fdd354ec1e963330890f9e0a92c8aecbe858",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-mLiteClinical-Base-135M-v1-onnx-android",
        revision: "9d6a95146005e741382513b81a9d2c795a69311e",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Korean-NomicMed-Large-395M-v1-onnx-android",
        revision: "583acfff64113fa08a6cd26a8a13a0a35c73caec",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-LiteClinical-Small-66M-v1",
        revision: "a6c3e2088a104d4b08034608347e4d26a4558b5d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-BioClinicalBERT-Base-110M-v1",
        revision: "3252a8885fefc2490a124c2be7ed7bce40088326",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-BioClinicalBERT-Base-110M-v1-mlx",
        revision: "d8aec713578f3de7f1f51eb1377d47b21aae08b4",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-LiteClinical-Small-66M-v1-onnx-android",
        revision: "48d93765f074085182b2d5e51eeaff79144846d5",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-BigMed-Large-278M-v1-onnx-android",
        revision: "725b64fcb13a9ccc263d4bb2d0c09b6dfbf7b54f",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-BioClinicalBERT-Base-110M-v1-onnx-android",
        revision: "cf72fb621ec4565d0e8fb067d0f4caed9ebfae98",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38347129440",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-BioClinicalModern-Base-149M-v1",
        revision: "11566ad69dafcde255aec256838464ba78d8b74f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38347129440",
    },
    // Batch 06 (run 38343611678, 24 of 24 rows; Hindi and Italian PII and
    // seven large DeBERTa-v2 fp16 exports, after the constant-MatMul fix).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-ClinicalNER-SuperClinical-Large-434M-v1-onnx-android",
        revision: "339a7b1d032b07c6de0e8623db54651b38743021",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-mLiteClinical-Base-135M-v1",
        revision: "bcacdbbca64f9abcef5756cdcf2df7d3b15f0243",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-SuperMedical-Base-125M-v1",
        revision: "b9ead47d9860cfb21279da726ed83996ad8bd94c",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-SuperClinical-Base-184M-v1",
        revision: "32c2bd645641913b7d424a800debec1ac424d2a3",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-SuperMedical-Large-355M-v1-onnx-android",
        revision: "102443508c3fbc4f23bd13808e7fbb88e7fa55f1",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-SuperClinical-Base-184M-v1-onnx-android",
        revision: "5bce6795c1faaf0c864f6a342f0e3d8240467f3c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-SuperMedical-Base-125M-v1-onnx-android",
        revision: "c5cb98f5d917d8d67c4dc43311dc55957f1696ab",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-SuperClinical-Small-44M-v1-onnx-android",
        revision: "c16ea0bd00711927b6b017149b2ffad1b62712ab",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-BioClinicalBERT-Base-110M-v1",
        revision: "11a61abb292002300cc95f6cc044a6ad12f370a4",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-mLiteClinical-Base-135M-v1-onnx-android",
        revision: "ab9b1aa1eba8940f3960e505436eb644c6dc1e60",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-BioClinicalBERT-Base-110M-v1-onnx-android",
        revision: "fe87e32b9a859c8a2a549ff352390a655628ae6b",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-BigMed-Large-278M-v1-onnx-android",
        revision: "b9aa7ae2244ed34cc2aabd740e7a9ddb68ccb808",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-SuperClinical-Small-44M-v1",
        revision: "e37905b6cfdaa5629017c0bbec9f37ebc6d7e1e3",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-SuperClinical-Large-434M-v1-onnx-android",
        revision: "7bc60fe823dadceb6c48c20535556b9a7c483cff",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-BioClinicalModern-Base-149M-v1-onnx-android",
        revision: "91770ca618e1b37f1fdffd354bc198bd0d85749c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-BioClinicalModern-Base-149M-v1",
        revision: "6909afdae3bdca433f8d12e6204230b9818b3346",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-BioClinicalModern-Large-395M-v1-onnx-android",
        revision: "8f3c2042353fbaa9347c695bf011057b89a94f60",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-ClinicalE5-Small-33M-v1",
        revision: "4a96ad80944ec76b2aef15f02ddf474d74fb8ce8",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-SuperClinical-Large-434M-v1-onnx-android",
        revision: "8d2db7712230956cd990e1bef35354b1f822a6fd",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-SuperClinical-Large-434M-v1-onnx-android",
        revision: "9bbf624d28f781f94d3276b814a423a4d4bd5de4",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-ClinicalE5-Small-33M-v1-onnx-android",
        revision: "1f0146e35c67acf076387f3e07d9d8043da1d821",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-BiomedBERT-Large-340M-v1-onnx-android",
        revision: "65444ceff48727811fd0ab450aced4b33a857ac0",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-SuperClinical-Large-434M-v1-onnx-android",
        revision: "6deef138dc53c3b227027216953ef57f936303ab",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38343611678",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-SuperClinical-Large-434M-v1-onnx-android",
        revision: "2bfdd0f83a9be257b63512be54373826119083c1",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38343611678",
    },
    // Batch 05 (run 38335975188, 24 of 24 rows; German and Hindi PII).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-SuperMedical-Base-125M-v1-onnx-android",
        revision: "494d3c99c7b572ef921dbab81933f145d063a1a0",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-mLiteClinical-Base-135M-v1-onnx-android",
        revision: "14e374f774a252e71c3480a72947f3c166544770",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-SuperMedical-Base-125M-v1",
        revision: "1d5ac702c791854e061c31c71f3df8b0611fcfef",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-SuperClinical-Base-184M-v1-onnx-android",
        revision: "10e62736331872b032ecebe01317def47ca8d024",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-BioClinicalBERT-Base-110M-v1-onnx-android",
        revision: "bce0c77d62cdf4984ba23d944a57a66a9c7e17aa",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-BioClinicalModern-Base-149M-v1-onnx-android",
        revision: "25fcabc733582cc8bf0e78bf7f34ee72d733efb5",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-SuperClinical-Small-44M-v1-onnx-android",
        revision: "f6a0d9ab98d09e3d8a723f1ae0ab951da4562caf",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-BigMed-Large-278M-v1-onnx-android",
        revision: "70ebdf2fe5ae8defa516c6ddef5aed5fbe10253c",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-SuperClinical-Small-44M-v1",
        revision: "b7145b13c5ebb0a1340459570579709346e7abbe",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-BioClinicalModern-Base-149M-v1",
        revision: "ec5f5116efa19dc9a9950a2b9b02425b7cc74c41",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-BioClinicalBERT-Base-110M-v1",
        revision: "95a0350a4139eb83271a83b57f57264526da508e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-SuperMedical-Large-355M-v1-onnx-android",
        revision: "bc992a722444081932103f0d3f888d98af6f911b",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-mLiteClinical-Base-135M-v1",
        revision: "e38aed6b8282b62885f08eab16ae32da88b2000a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-mSuperClinical-Large-279M-v1-onnx-android",
        revision: "a5c1abb598a7d81ec183da74de0ee1e89c4f0365",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-ClinicalE5-Small-33M-v1-onnx-android",
        revision: "7f1dab9ef20741988c89109d2b81a504a387f5ca",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-BioClinicalModern-Large-395M-v1-onnx-android",
        revision: "95b785e8258206f7a5c6c15b4f36a72c893c507c",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-ClinicalE5-Small-33M-v1",
        revision: "a8d1838951bc73440429c395c6a69f8b20b83a29",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-EuroMed-Large-210M-v1-onnx-android",
        revision: "b8d4136ed5f18cf8b79c54d29d43049171e2cac3",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-ClinicalLongformer-Base-149M-v1",
        revision: "03068aa0d56bbbf68ebf0fc86d79a5a49d1d098b",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-BiomedBERT-Large-340M-v1-onnx-android",
        revision: "b633d96469a4f35a7c5b92f0a0624d0446bdf75e",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-FastClinical-Small-82M-v1",
        revision: "e2afc9b073e6dd955068d92ab46df353b7009436",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-LiteClinical-Small-66M-v1",
        revision: "8db1ce65f78044fe768f8a71ab5c300b29f92778",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-FastClinical-Small-82M-v1-onnx-android",
        revision: "aa3067d2f190587ee8a4bad2b13bc3d0d70d262c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38335975188",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-LiteClinical-Small-66M-v1-onnx-android",
        revision: "600171889bc231274df1b32303813342edfffc77",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38335975188",
    },
    // Batch 04 (run 38331707946, 24 of 24 rows; French and German PII).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-SuperMedical-Base-125M-v1-onnx-android",
        revision: "79e32d475218f080381a18b8998feab6d2fcef6c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-SuperMedical-Large-355M-v1-onnx-android",
        revision: "ea641599fda5a7cae46873c87ea7322ab0a342a1",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-mLiteClinical-Base-135M-v1",
        revision: "515f4e552e234db0ce5b7f1a7d363f976a77160c",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-SuperClinical-Small-44M-v1-onnx-android",
        revision: "de66f964fc3182f42a527e3c66733a2e2ae2fd67",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-SuperMedical-Base-125M-v1",
        revision: "9931d98e75d2b20ea651ab98631e639bb022eece",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-BioClinicalBERT-Base-110M-v1",
        revision: "2fc7c9eb88824e2e62ac62b4e3e977bbf92e47e6",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-SuperClinical-Small-44M-v1",
        revision: "cc0bb5689568519fd12a106afd74f434c6c49fa0",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-mSuperClinical-Large-279M-v1-onnx-android",
        revision: "612d6a11fea6dea30fcf574400d3bb70dc2d4641",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-BioClinicalModern-Large-395M-v1-onnx-android",
        revision: "ea8ed48068ab2dd78dc9a388d59103b99b6032aa",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-BigMed-Large-278M-v1-onnx-android",
        revision: "886203162dbf4e3c7b7762119a6bd37faac6f039",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-BioClinicalBERT-Base-110M-v1-onnx-android",
        revision: "48fa77fbdbf34e74fbf629d1bc57eb3bee8bf12d",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-BioClinicalModern-Base-149M-v1",
        revision: "43de4f2f2249ad14a3f0920268c8df7021084f12",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-BioClinicalModern-Base-149M-v1-onnx-android",
        revision: "ba5a95ed3f27f87eb93c91b8629723cbba107de8",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-mLiteClinical-Base-135M-v1-onnx-android",
        revision: "5fcf2676171db8bdd8c603760094834aa9729507",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-ClinicalE5-Small-33M-v1",
        revision: "e7bda7ff2ef190c68cc041a960f9935d218762ff",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-BiomedBERT-Large-340M-v1-onnx-android",
        revision: "b4a09628610cd4279e25229c6ce1243c90fab005",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-ClinicalE5-Small-33M-v1-onnx-android",
        revision: "e37d973e98c7c35f3f0f40e5c990da2c589e8be9",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-FastClinical-Small-82M-v1",
        revision: "cf8654cce1ad4214b10a97545b9a515896a1971f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-FastClinical-Small-82M-v1-onnx-android",
        revision: "26bec9f17b99d469a956904d37763889355c5692",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-ClinicalLongformer-Base-149M-v1",
        revision: "7f4248dc0057177c325a78d7d7200f88f626d592",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-LiteClinical-Small-66M-v1",
        revision: "92de5d8d1a13eceb2e3ffb274be6a8d7157c1bc3",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-LiteClinical-Small-66M-v1-onnx-android",
        revision: "eabb097e8af335f7ba491d274f4db3d4cf9c5045",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-EuroMed-Large-210M-v1-onnx-android",
        revision: "eda2df4a92ea606fe9eb244dfef6a8b1d84e83b1",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38331707946",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-SuperClinical-Base-184M-v1",
        revision: "3c3f1b0a48f4d1758a95c3d3bfd39bfed6d6fc95",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38331707946",
    },
    // Batch 03 (run 38320247418, 21 of 21 rows; includes the three
    // Transformers 5 configs batch 02 could not load).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-BiomedBERT-Large-340M-v1-onnx-android",
        revision: "442c56729201d2492b6316249ab23cd525cc3c2d",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-SuperMedical-Base-125M-v1",
        revision: "8920d465158fb63d4d1e7c3ac9ef6a4082da8d9f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-ClinicalE5-Small-33M-v1",
        revision: "9b4b6504b77dd1d3f7ef5c40f719adb1a0e8f7d1",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-BioClinicalModern-Base-149M-v1",
        revision: "d21ec33f198e192a087535b6ec3b23704c42f7ae",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-BigMed-Large-278M-v1-onnx-android",
        revision: "d41dd08c54da5cf3cb9519a0f3ceb27d7a8df41c",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-BioClinicalModern-Base-149M-v1-onnx-android",
        revision: "4a131943b1125d188ad9c41d004a268b61138b18",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-ClinicalLongformer-Base-149M-v1",
        revision: "ce00777b0a19bf18d57f430ff9fa404ab9502383",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-ClinicalLongformer-Base-149M-v1",
        revision: "a1237b365b6d4f399a11b48528d93c2b965e151d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-BioClinicalModern-Base-149M-v1",
        revision: "561a0eb232449d55dd11de52812cbb555163fb23",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-FastClinical-Small-82M-v1",
        revision: "cfb4de5f42e917a51684a0694e02f1f0506cc25f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-BioClinicalBERT-Base-110M-v1",
        revision: "11f77a4af1888daa928dccff3be6441b1e0eb880",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-ClinicalE5-Small-33M-v1-onnx-android",
        revision: "1e2125663db5ada7936e79dd276cbe75c08a9bc2",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-BioClinicalModern-Large-395M-v1-onnx-android",
        revision: "783ff5c2fdd37e79baa16985bb8f32944ec19b88",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-BioClinicalBERT-Base-110M-v1-onnx-android",
        revision: "afe1521cadf478ade3635ed07ef6ec30754ddd36",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-FastClinical-Small-82M-v1",
        revision: "04d79bfe5e5e21594f36e7ae079ea9bc8ce44452",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-EuroMed-Large-210M-v1-onnx-android",
        revision: "c6348a95d88bc3ac950b58018358d73caadda150",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-LiteClinical-Small-66M-v1",
        revision: "d068f2d8a231e0e11e34fa4fac216c345474a0da",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-SuperClinical-Base-184M-v1",
        revision: "7464a44a6a35704309c3a17e49e0ebb1416d8f6a",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-LiteClinical-Small-66M-v1-onnx-android",
        revision: "d3b631fe94f0e5ae070325f2ed0a216677121234",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-FastClinical-Small-82M-v1-onnx-android",
        revision: "07af2f29060833c19ec0f77b8ffaec1bae919a79",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38320247418",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-SuperClinical-Base-184M-v1-onnx-android",
        revision: "5f1dd294a8806577ee2a42939441e7f62dd9ab6c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38320247418",
    },
    // Batch 02 (run 38314653881, 21 of 24 rows; includes the legacy
    // gamma/beta Dutch BioClinicalBERT requalified).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-BioClinicalModern-Base-149M-v1-onnx-android",
        revision: "583fa3a7195c9f4f4ec78a1dcc43002758b06d74",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-BiomedBERT-Large-340M-v1-onnx-android",
        revision: "435e8d2f9502d80d5df6b6b863fee2c511499fe1",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-ClinicalE5-Small-33M-v1",
        revision: "ee25778d9ba2b79dda12c3b40719cf9f7d7601a6",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Chinese-BigMed-Large-278M-v1-onnx-android",
        revision: "57ef1c36793f38f6fa6b914c84409ca132efb5aa",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-EuroMed-Large-210M-v1-onnx-android",
        revision: "5dc33894ca53209ce3d42ae00a8b944bb8ef18e5",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-BioClinicalBERT-Base-110M-v1",
        revision: "973bfed6de33ed2a8a77618fc30b8ae5b55af878",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-BioClinicalModern-Large-395M-v1-onnx-android",
        revision: "f5f172ed0a4880bd65e192e024a4a4847dbc3c3a",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-LiteClinical-Small-66M-v1-onnx-android",
        revision: "5f0683137d22fd80784da850d9a9e50dc7d70bba",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-BioClinicalBERT-Base-110M-v1-onnx-android",
        revision: "a78102a6b8339fece9e8810f073d13e28554bc2e",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-FastClinical-Small-82M-v1-onnx-android",
        revision: "e31796967bd1c84e13d0b62ad8ad28f91e07aa8a",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-ClinicalE5-Small-33M-v1-onnx-android",
        revision: "27ff9e2c867ed94bcaff31bd4007417ea9a54812",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-LiteClinical-Small-66M-v1",
        revision: "0f6930e3db8d002b37f4c45689d45c52424721f0",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-SuperClinical-Base-184M-v1",
        revision: "9cd3176bd602649e62f8dfb28b01f7bce5005080",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-SuperClinical-Base-184M-v1-onnx-android",
        revision: "a7bf385f792274c5cf68eb2a6ff99699fddb23c9",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-SuperMedical-Base-125M-v1-onnx-android",
        revision: "b33e9db79161a9110723aa89a5052c7f53d7c3bb",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-mLiteClinical-Base-135M-v1",
        revision: "5159368ec120bf24a94148255b051ed1dfb4d22d",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-SuperMedical-Large-355M-v1-onnx-android",
        revision: "19b9df028caf1b245bf5db3dbd23b292428ceea6",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-mSuperClinical-Large-279M-v1-onnx-android",
        revision: "b07d15f70f04db2598cd7c18f00f1f98f4b05442",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-mLiteClinical-Base-135M-v1-onnx-android",
        revision: "8fa91cec008fc92a64430a1e12b410e4f386f8fb",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-FastClinical-Small-82M-v1-onnx-android",
        revision: "0eb78b1e60c11092bd67f2c1da08f2c81f9a6282",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38314653881",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-FastClinical-Small-82M-v1",
        revision: "85d5bbaf4f5fcf2d04da9e4a1812d9c76386c72e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38314653881",
    },
    // Batch 01 of the per-row harness (run 38301622121, 16 of 24 rows;
    // failures are recorded as negative evidence in the capability matrix).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-SuperMedical-125M",
        revision: "290a4a9bfc861153b10c0677633823bf48a43053",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-SuperMedical-125M-mlx",
        revision: "de25997d7fa4759a9589b83eede578e06a8827c4",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-BioClinicalBERT-Base-110M-v1-onnx-android",
        revision: "13ea8ec34505ea36440a9b3fe98b7d11c0554dbb",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Bengali-mSuperClinical-Large-279M-v1-onnx-android",
        revision: "b0a7d247d4b92db7a73ce15ae997d6bab3b6bf53",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-SuperClinical-141M-v1-onnx-android",
        revision: "4846f43de502821bc1db07fcbfbfb4a388f4a441",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-SuperClinical-141M",
        revision: "3539a0dd65bb409dd4259d8ef7e3305294214a30",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-EuroMed-212M-v1-onnx-android",
        revision: "7a4e1f1b17e103ca5b18ef8c032b0860c3065efe",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-SuperMedical-125M-v1-onnx-android",
        revision: "e6fa586892557eea153756f1b811fb34cd676002",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-BioClinicalModern-Base-149M-v1",
        revision: "732b1c5c20dd55371f67679010500e67dc966bff",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-BioClinicalBERT-Base-110M-v1",
        revision: "87c4b15a7647ad51ad1f4a1f05ea95ae464bb54f",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-BioClinicalModern-Base-149M-v1-onnx-android",
        revision: "7d2b9fe28fe69f8b94830499775e46456c623288",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-BioClinicalModern-Large-395M-v1-onnx-android",
        revision: "5f0e5f4c75425f422b93a29b25cb184eb38fa463",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-ClinicalE5-Small-33M-v1",
        revision: "2ea947130eb28d610ceb96472af0b694fce9c344",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-BiomedBERT-Large-340M-v1-onnx-android",
        revision: "0bc1f3783c4618d2fb7c573fbe5bdce9c20c579d",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-ClinicalE5-Small-33M-v1-onnx-android",
        revision: "79f7db205869b1be4be23ac4f42aa95bdedc5aee",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38301622121",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-BigMed-Large-278M-v1-onnx-android",
        revision: "5811d0ca66f0c93e66b1de5b10754ceef5717a53",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "38301622121",
    },
    // Clinical Longformer on the exact candle local-window path; entities
    // match Hugging Face LongformerForTokenClassification (run 38300309054).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-ClinicalLongformer-Base-149M-v1",
        revision: "ade4412daadbae377eda99f73e107557d15d31c0",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38300309054",
    },
    // The remaining QwenMed PII rows, int8 export (run 38260759126).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "b23b67ba6797bd57fe43ab5c74dbb02370261970",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "ab281a758ae9713c53e1b230342ef52e5ab4e053",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "236025090d28429f45664edd8b43bc8fd4903511",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "8eb77f446e812328a8c461526e03a2f195f7216e",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "cb463fd8f72a1340fbf81f49be9adb4c50c17ebf",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Japanese-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "2652ad2ff21b8ff9bed70028617a5e034dd618d9",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Korean-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "d15e599e481265938fd8e49a8a10b4ae4ca91f16",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "b9b80f6b92aa18d5726de026b0555c6bdf34daa3",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "e47526af75653006cb2ae0200dd271c733dacba0",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "e4c4c8aa8c55089bfa26b337dc8257356031e58f",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "19ad9f710bb9d442a745abbfea7030115c7fd187",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "d796a5e3366f65fa26e4a857bbfc010f800b7213",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    // Catalog rows without an `architecture` field; their `config.json`
    // declares bert, roberta, deberta-v2 or modernbert (run 38163293053).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-BioMed-109M",
        revision: "49bedb870db7008773880b643733c78100737e12",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38163293053",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-SuperMedical-355M",
        revision: "ee4f7b5d9e41a005f35a9ed033a77de71c68e4e5",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38163293053",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-SuperClinical-184M",
        revision: "3d01b67e46890c10540daa00b57693e6238ebb46",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38163293053",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-BioPatient-108M",
        revision: "6ffc399d133a28adc6b9a835f351bc2e6f8fc73c",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38163293053",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ProteinDetect-PubMed-v2-109M",
        revision: "f3877242a44ac750054eafc097bc92e55368b6a4",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38163293053",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-SpeciesDetect-ModernMed-395M",
        revision: "8e126e50c965925235b26401a44e865d6c2fea18",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38163293053",
    },
    // GLiNER zero-shot MLX exports (Spec 104), run with synthetic labels.
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-ZeroShot-NER-Anatomy-Small-166M-mlx",
        revision: "02a3c8ba849c2ddcd5f994ce882df2418abbd02e",
        runtime: "candle_gliner",
        weight_file: "weights.safetensors",
        evidence_run: "38161626788",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-ZeroShot-NER-Disease-Medium-209M-mlx",
        revision: "0c3d802dcba6c88afa3fd6b6451216b4d56c1b1a",
        runtime: "candle_gliner",
        weight_file: "weights.safetensors",
        evidence_run: "38163323960",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-EuroMed-Large-210M-v1-onnx-android",
        revision: "6f15663a6e6f863792f9ad7ef1725552c703dabb",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38152456058",
    },
    // Only the int8 export fits the 1 GiB artifact bound (model.onnx keeps
    // its weights in external data; model_fp16.onnx is 1.19 GB).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Chinese-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "0f9550e49fbf1d90d9136879be1a635b5e9728f2",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38152456058",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-33M-v1-onnx-android",
        revision: "54c9cc119d325ddad23300a1e9108bafa9643a06",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "37984399447",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M-v1-onnx-android",
        revision: "ad5e79cb87caf49d9ecc28da4de008856527ab63",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "37984399447",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-82M-v1-onnx-android",
        revision: "95f0bf742c8c31d24b66190e17c1add82664482c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "37984399447",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ModernClinical-149M-v1-onnx-android",
        revision: "66525717fb59d0ff6e1396b7c2d1ab8c1e4a93c8",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "37984399447",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-SuperClinical-Small-44M-v1-onnx-android",
        revision: "e41c8009026fdd98dae146cb67ae9b16673faf49",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "37984399447",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-BigMed-Large-278M-v1-onnx-android",
        revision: "da3fec609614401bba850912065c832a74586d5a",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "37984399447",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-33M",
        revision: "3c914174cf099fda326b43084f679ad31425f88c",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "37986892863",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M",
        revision: "1ab9359dada59aafc76bf63879c4984a5fa3490e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "37986892863",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-82M",
        revision: "724a3702f1fcc80ab0bd695f1a3e3ddf2ac854f1",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "37986892863",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-SuperClinical-Small-44M-v1",
        revision: "ee9cafadbe5689d779f4ae6971d30eb348274b82",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "37986892863",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ModernClinical-149M",
        revision: "3c0bdcc332af0fc54b8f945bcfbebdd375eb69dd",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "37986892863",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-33M-mlx",
        revision: "70bcf3d3c4e37781723795830772d4a4ad8f2f1c",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38039625442",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M-mlx",
        revision: "1be81258e117a95f41600bd2b0b5684581a5714a",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38039625442",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-82M-mlx",
        revision: "30ae97175384e04e2c961ab6dc6af5bc0342d3ce",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38039625442",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-SuperClinical-141M-mlx",
        revision: "8d9d9946e0b291bffb505a59b7f73c15ef4a92be",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38041300631",
    },
];

/// Recorded execution evidence for one catalog row, if any.
#[must_use]
pub fn executed_model(repo_id: &str) -> Option<&'static ExecutedModel> {
    EXECUTED_MODELS.iter().find(|m| m.repo_id == repo_id)
}

/// Architectures recorded as not supported by the current runtime, with the cause.
pub const KNOWN_UNSUPPORTED: &[(&str, &str)] = &[];

/// Architectures executed end to end through the native safetensors runtime
/// (Spec 104, qualification run 37986892863) from PyTorch-format rows.
pub const SAFETENSORS_EVIDENCED_ARCHITECTURES: &[&str] =
    &["bert", "distilbert", "roberta", "deberta-v2", "modernbert"];

/// Architectures executed from OpenMed MLX `mlx-fp` exports (renamed-key
/// safetensors, `mlx_layout.rs`): qualification runs 38039625442 (bert,
/// distilbert, roberta) and 38041300631 (deberta-v2). Repositories that ship
/// no `tokenizer.json` are refused at acquisition.
pub const MLX_EVIDENCED_ARCHITECTURES: &[&str] = &["bert", "distilbert", "roberta", "deberta-v2"];

/// PyTorch-format architectures known not to fit the safetensors path.
pub const SAFETENSORS_KNOWN_UNSUPPORTED: &[(&str, &str)] = &[
    (
        "xlm-roberta",
        "PyTorch-format XLM-R weights (1.11 GB for the 278M-parameter models) exceed the 1 GiB Pack bound",
    ),
    (
        "gliner",
        "PyTorch GLiNER repositories ship pickle weights only (pytorch_model.bin), which are refused; the ZeroShot MLX exports run",
    ),
    (
        "eurobert",
        "the candle runtime has no EuroBERT encoder; the ONNX exports run on tract",
    ),
    (
        "qwen",
        "the candle runtime has no Qwen token-classification encoder; the int8 ONNX export runs on tract",
    ),
];

/// Architecture families known not to run on any admitted runtime, matched by
/// a substring of the catalog `architecture` (OpenMed names some families per
/// repository, for example `openmed-pii-clinicallongformer-base-149m-v1`).
pub const KNOWN_UNSUPPORTED_FAMILIES: &[(&str, &str)] = &[
    (
        "longformer",
        "Longformer ONNX export: tract 0.22.4 fails in Gather at 512 and 1024 tokens, after the omitted Pad input is dropped (run 38172472172); the PyTorch-format rows run on candle (exact local-window path, run 38300309054)",
    ),
    (
        "privacy-filter",
        "privacy-filter weights are 1.49 GB (8-bit MLX, quantized) to 2.80 GB, over the 1 GiB Pack bound",
    ),
];

/// Published exports known to be defective, matched by architecture and a
/// repository-id prefix, with the cause.
/// Why a listed export in [`KNOWN_DEFECTIVE_EXPORTS`] cannot run.
pub const DEGENERATE_TOKENIZER_REASON: &str = "the published tokenizer.json is a BPE model with zero merges, so text is split into single characters; the runtime refuses it";

/// Published exports known to be defective, by exact repository id. The 28
/// XLM-R exports whose `tokenizer.json` is BPE with an empty `merges` list
/// (26 NER, 2 PII), found by reading the end of every XLM-R ONNX row's
/// tokenizer (2026-10-13; batch run 38301622121 confirmed it on
/// `OpenMed-PII-BigMed-Large-278M-v1`). The other 39 XLM-R NER exports ship a
/// well-formed Unigram tokenizer and are not listed. The runtime refuses a
/// degenerate tokenizer at prepare in any case.
pub const KNOWN_DEFECTIVE_EXPORTS: &[&str] = &[
    "OpenMed/OpenMed-NER-AnatomyDetect-BigMed-278M-v1-onnx-android",
    "OpenMed/OpenMed-NER-AnatomyDetect-BigMed-560M-v1-onnx-android",
    "OpenMed/OpenMed-NER-BloodCancerDetect-BigMed-278M-v1-onnx-android",
    "OpenMed/OpenMed-NER-BloodCancerDetect-BigMed-560M-v1-onnx-android",
    "OpenMed/OpenMed-NER-ChemicalDetect-BigMed-278M-v1-onnx-android",
    "OpenMed/OpenMed-NER-ChemicalDetect-BigMed-560M-v1-onnx-android",
    "OpenMed/OpenMed-NER-DNADetect-BigMed-278M-v1-onnx-android",
    "OpenMed/OpenMed-NER-DNADetect-BigMed-560M-v1-onnx-android",
    "OpenMed/OpenMed-NER-DiseaseDetect-BigMed-278M-v1-onnx-android",
    "OpenMed/OpenMed-NER-DiseaseDetect-BigMed-560M-v1-onnx-android",
    "OpenMed/OpenMed-NER-GenomeDetect-BigMed-278M-v1-onnx-android",
    "OpenMed/OpenMed-NER-GenomeDetect-BigMed-560M-v1-onnx-android",
    "OpenMed/OpenMed-NER-GenomicDetect-BigMed-278M-v1-onnx-android",
    "OpenMed/OpenMed-NER-GenomicDetect-BigMed-560M-v1-onnx-android",
    "OpenMed/OpenMed-NER-OncologyDetect-BigMed-278M-v1-onnx-android",
    "OpenMed/OpenMed-NER-OncologyDetect-BigMed-560M-v1-onnx-android",
    "OpenMed/OpenMed-NER-OrganismDetect-BigMed-278M-v1-onnx-android",
    "OpenMed/OpenMed-NER-OrganismDetect-BigMed-560M-v1-onnx-android",
    "OpenMed/OpenMed-NER-PathologyDetect-BigMed-278M-v1-onnx-android",
    "OpenMed/OpenMed-NER-PathologyDetect-BigMed-560M-v1-onnx-android",
    "OpenMed/OpenMed-NER-PharmaDetect-BigMed-278M-v1-onnx-android",
    "OpenMed/OpenMed-NER-PharmaDetect-BigMed-560M-v1-onnx-android",
    "OpenMed/OpenMed-NER-ProteinDetect-BigMed-278M-v1-onnx-android",
    "OpenMed/OpenMed-NER-ProteinDetect-BigMed-560M-v1-onnx-android",
    "OpenMed/OpenMed-NER-SpeciesDetect-BigMed-278M-v1-onnx-android",
    "OpenMed/OpenMed-NER-SpeciesDetect-BigMed-560M-v1-onnx-android",
    "OpenMed/OpenMed-PII-BigMed-Large-278M-v1-onnx-android",
    "OpenMed/OpenMed-PII-BigMed-Large-560M-v1-onnx-android",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeExpectation {
    /// This exact row was executed end to end on its real weights (see
    /// [`EXECUTED_MODELS`]); evidence, not task qualification.
    ExecutedTested {
        runtime: String,
        revision: String,
        weight_file: String,
        evidence_run: String,
    },
    /// Same task, an ONNX file, and an architecture with recorded execution
    /// evidence. Expected to run; still unverified for this row.
    ExpectedRunnable,
    /// Known not to run on the current runtime.
    KnownUnsupported { reason: String },
    /// No ONNX file, but PyTorch-format weights for an architecture with
    /// recorded native safetensors execution evidence (Spec 104). Expected to
    /// run; still unverified for this row (its weights may be pickle-only,
    /// which acquisition refuses).
    ExpectedRunnableSafetensors,
    /// OpenMed zero-shot MLX export (GLiNER uni-encoder span, Spec 104) in a
    /// tier whose weights fit the 1 GiB Pack bound. Expected to run through
    /// the GLiNER runtime with caller-supplied labels; still unverified for
    /// this row.
    ExpectedRunnableZeroShot,
    /// No runnable artifact is listed (for example MLX only).
    NoOnnxArtifact,
    /// The task is not token classification.
    UnsupportedTask { task: String },
    /// ONNX token classifier with an architecture not yet tried.
    Untested,
    /// The architecture runs, but even a half-precision export of this many
    /// parameters (2 bytes each) would exceed the 1 GiB Pack artifact bound.
    /// An estimate from the catalog parameter count, not a measurement; a
    /// recorded run of the row (for example a fitting int8 export) overrides
    /// it.
    EstimatedOverBound { estimated_megabytes: u64 },
}

/// Expectation for `row` on the local ONNX token-classification runtime.
/// Expectation for an OpenMed MLX repository (no ONNX, MLX weights only).
fn mlx_expectation(row: &CatalogRow, arch: &str) -> RuntimeExpectation {
    let full_precision = row.formats.iter().any(|f| f.eq_ignore_ascii_case("mlx-fp"));
    if !full_precision {
        return RuntimeExpectation::KnownUnsupported {
            reason: "quantized MLX exports change tensor contents and are not supported".into(),
        };
    }
    // Zero-shot repositories use GLiNER span heads (Spec 104). In OpenMed
    // v3.0.0 the Small (611 MB) and Medium (781 MB) exports fit the Pack
    // bound; the untiered Base (1.16 GB) and Large (1.78 GB) ones do not.
    if row.repo_id.contains("ZeroShot") {
        if arch != "deberta-v2" {
            return RuntimeExpectation::Untested;
        }
        return match row.tier.as_deref() {
            Some(t) if t.eq_ignore_ascii_case("small") || t.eq_ignore_ascii_case("medium") => {
                RuntimeExpectation::ExpectedRunnableZeroShot
            }
            _ => RuntimeExpectation::KnownUnsupported {
                reason: "zero-shot MLX exports outside the Small and Medium tiers are 1.16-1.78 GB, over the 1 GiB Pack bound".into(),
            },
        };
    }
    if let Some((_, reason)) = SAFETENSORS_KNOWN_UNSUPPORTED
        .iter()
        .find(|(a, _)| *a == arch)
    {
        return RuntimeExpectation::KnownUnsupported {
            reason: (*reason).to_string(),
        };
    }
    if MLX_EVIDENCED_ARCHITECTURES.contains(&arch) {
        return RuntimeExpectation::ExpectedRunnableSafetensors;
    }
    RuntimeExpectation::Untested
}

/// Bytes per parameter of the smallest export MedScale admits for a runnable
/// architecture (fp16); fp32 exports are larger, int8 exports vary.
const MIN_BYTES_PER_PARAMETER: u64 = 2;
const PACK_ARTIFACT_BOUND: u64 = 1 << 30;

/// Downgrades an `Expected*` result when the parameter count says no
/// half-precision export can fit the bound.
fn size_checked(row: &CatalogRow, expectation: RuntimeExpectation) -> RuntimeExpectation {
    let expected = matches!(
        expectation,
        RuntimeExpectation::ExpectedRunnable
            | RuntimeExpectation::ExpectedRunnableSafetensors
            | RuntimeExpectation::ExpectedRunnableZeroShot
    );
    match row.param_count {
        Some(params)
            if expected && params.saturating_mul(MIN_BYTES_PER_PARAMETER) > PACK_ARTIFACT_BOUND =>
        {
            RuntimeExpectation::EstimatedOverBound {
                estimated_megabytes: params.saturating_mul(MIN_BYTES_PER_PARAMETER) / 1_000_000,
            }
        }
        _ => expectation,
    }
}

pub fn runtime_expectation(row: &CatalogRow) -> RuntimeExpectation {
    if executed_model(&row.repo_id).is_some() || row.task != "token-classification" {
        return expectation_without_size(row);
    }
    size_checked(row, expectation_without_size(row))
}

fn expectation_without_size(row: &CatalogRow) -> RuntimeExpectation {
    if row.task != "token-classification" {
        return RuntimeExpectation::UnsupportedTask {
            task: row.task.clone(),
        };
    }
    if let Some(model) = executed_model(&row.repo_id) {
        return RuntimeExpectation::ExecutedTested {
            runtime: model.runtime.to_owned(),
            revision: model.revision.to_owned(),
            weight_file: model.weight_file.to_owned(),
            evidence_run: model.evidence_run.to_owned(),
        };
    }
    let arch = row.architecture.to_ascii_lowercase();
    let has_onnx = row.formats.iter().any(|f| f.eq_ignore_ascii_case("onnx"));
    // Longformer: the ONNX export is blocked on tract, but PyTorch-format
    // rows run on the exact candle local-window path (Spec 103 T103-19).
    if arch.contains("longformer") && !has_onnx {
        return RuntimeExpectation::ExpectedRunnableSafetensors;
    }
    if let Some((_, reason)) = KNOWN_UNSUPPORTED_FAMILIES
        .iter()
        .find(|(family, _)| arch.contains(family))
    {
        return RuntimeExpectation::KnownUnsupported {
            reason: (*reason).to_string(),
        };
    }
    if !row.formats.iter().any(|f| f.eq_ignore_ascii_case("onnx")) {
        // OpenMed v3.0.0 lists MLX repositories (`*-mlx`) with formats such as
        // `["mlx-fp", "pytorch"]`; they publish MLX weights only
        // (`weights.safetensors` with renamed keys, see `mlx_layout`).
        let mlx = row
            .formats
            .iter()
            .any(|f| f.to_ascii_lowercase().starts_with("mlx"));
        if mlx {
            return mlx_expectation(row, &arch);
        }
        if row
            .formats
            .iter()
            .any(|f| f.eq_ignore_ascii_case("pytorch"))
        {
            if let Some((_, reason)) = SAFETENSORS_KNOWN_UNSUPPORTED
                .iter()
                .find(|(a, _)| *a == arch)
            {
                return RuntimeExpectation::KnownUnsupported {
                    reason: (*reason).to_string(),
                };
            }
            if SAFETENSORS_EVIDENCED_ARCHITECTURES.contains(&arch.as_str()) {
                return RuntimeExpectation::ExpectedRunnableSafetensors;
            }
            return RuntimeExpectation::Untested;
        }
        return RuntimeExpectation::NoOnnxArtifact;
    }
    if KNOWN_DEFECTIVE_EXPORTS.contains(&row.repo_id.as_str()) {
        return RuntimeExpectation::KnownUnsupported {
            reason: DEGENERATE_TOKENIZER_REASON.to_owned(),
        };
    }
    if let Some((_, reason)) = KNOWN_UNSUPPORTED.iter().find(|(a, _)| *a == arch) {
        return RuntimeExpectation::KnownUnsupported {
            reason: (*reason).to_string(),
        };
    }
    if EVIDENCED_ARCHITECTURES.contains(&arch.as_str()) {
        return RuntimeExpectation::ExpectedRunnable;
    }
    RuntimeExpectation::Untested
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::ModelCatalog;

    const COMMIT: &str = "ea920f36fadd7b45935247d639f0ffa1ef493b23";

    fn row(task: &str, arch: &str, formats: &str) -> CatalogRow {
        named("Org/m", task, arch, formats)
    }

    fn named(repo: &str, task: &str, arch: &str, formats: &str) -> CatalogRow {
        let line = format!(
            r#"{{"repo_id":"{repo}","family":"NER","task":"{task}","architecture":"{arch}","formats":{formats},"license":"apache-2.0"}}"#
        );
        ModelCatalog::import("s/f", COMMIT, line.as_bytes())
            .unwrap()
            .get(repo)
            .unwrap()
            .clone()
    }

    #[test]
    fn expectations_follow_recorded_evidence() {
        assert_eq!(
            runtime_expectation(&row("token-classification", "bert", r#"["onnx"]"#)),
            RuntimeExpectation::ExpectedRunnable
        );
        assert_eq!(
            runtime_expectation(&row(
                "token-classification",
                "ModernBERT",
                r#"["pytorch","onnx"]"#
            )),
            RuntimeExpectation::ExpectedRunnable
        );
        assert_eq!(
            runtime_expectation(&row("token-classification", "deberta-v2", r#"["onnx"]"#)),
            RuntimeExpectation::ExpectedRunnable
        );
        assert_eq!(
            // An XLM-R export outside the exact defective list and without
            // its own run (synthetic name).
            runtime_expectation(&named(
                "OpenMed/OpenMed-PII-Example-BigMed-Large-278M-v1-onnx-android",
                "token-classification",
                "xlm-roberta",
                r#"["onnx"]"#
            )),
            RuntimeExpectation::ExpectedRunnable
        );
        assert!(matches!(
            runtime_expectation(&named(
                "OpenMed/OpenMed-NER-DiseaseDetect-BigMed-278M-v1-onnx-android",
                "token-classification",
                "xlm-roberta",
                r#"["onnx"]"#
            )),
            RuntimeExpectation::KnownUnsupported { .. }
        ));
        assert_eq!(
            runtime_expectation(&row("token-classification", "bert", r#"["mlx-fp"]"#)),
            RuntimeExpectation::ExpectedRunnableSafetensors
        );
        assert_eq!(
            runtime_expectation(&row("token-classification", "deberta-v2", r#"["pytorch"]"#)),
            RuntimeExpectation::ExpectedRunnableSafetensors
        );
        assert!(matches!(
            runtime_expectation(&row(
                "token-classification",
                "xlm-roberta",
                r#"["pytorch"]"#
            )),
            RuntimeExpectation::KnownUnsupported { .. }
        ));
        // Pickle-only GLiNER, and encoders the candle runtime lacks.
        for arch in ["gliner", "eurobert", "qwen"] {
            assert!(
                matches!(
                    runtime_expectation(&row("token-classification", arch, r#"["pytorch"]"#)),
                    RuntimeExpectation::KnownUnsupported { .. }
                ),
                "{arch}"
            );
        }
        assert_eq!(
            runtime_expectation(&row(
                "token-classification",
                "bert",
                r#"["mlx-fp","pytorch"]"#
            )),
            RuntimeExpectation::ExpectedRunnableSafetensors
        );
        assert!(matches!(
            runtime_expectation(&row("token-classification", "bert", r#"["mlx-8bit"]"#)),
            RuntimeExpectation::KnownUnsupported { .. }
        ));
        assert!(matches!(
            runtime_expectation(&row("token-classification", "xlm-roberta", r#"["mlx-fp"]"#)),
            RuntimeExpectation::KnownUnsupported { .. }
        ));
        assert_eq!(
            runtime_expectation(&row("token-classification", "modernbert", r#"["mlx-fp"]"#)),
            RuntimeExpectation::Untested
        );
        assert_eq!(
            runtime_expectation(&row("token-classification", "gliner", r#"["onnx"]"#)),
            RuntimeExpectation::Untested
        );
        assert_eq!(
            runtime_expectation(&row("text-generation", "qwen", r#"["onnx"]"#)),
            RuntimeExpectation::UnsupportedTask {
                task: "text-generation".into()
            }
        );
    }

    #[test]
    fn longformer_pytorch_rows_run_on_candle_while_onnx_rows_stay_blocked() {
        assert_eq!(
            runtime_expectation(&row(
                "token-classification",
                "openmed-pii-dutch-clinicallongformer-base-149m-v1",
                r#"["pytorch"]"#
            )),
            RuntimeExpectation::ExpectedRunnableSafetensors
        );
        assert!(matches!(
            runtime_expectation(&row(
                "token-classification",
                "openmed-pii-dutch-clinicallongformer-base-149m-v1",
                r#"["onnx"]"#
            )),
            RuntimeExpectation::KnownUnsupported { .. }
        ));
    }

    #[test]
    fn known_unsupported_families_match_per_repository_architecture_names() {
        for (arch, formats) in [
            ("openmed-pii-clinicallongformer-base-149m-v1", r#"["onnx"]"#),
            ("clinical-longformer", r#"["onnx"]"#),
            ("privacy-filter-multilingual-v2", r#"["mlx-fp","pytorch"]"#),
        ] {
            assert!(
                matches!(
                    runtime_expectation(&row("token-classification", arch, formats)),
                    RuntimeExpectation::KnownUnsupported { .. }
                ),
                "{arch}"
            );
        }
        // A catalog row without an architecture that was executed reports it.
        let r = named(
            "OpenMed/OpenMed-NER-AnatomyDetect-BioMed-109M",
            "token-classification",
            "",
            r#"["pytorch"]"#,
        );
        assert!(matches!(
            runtime_expectation(&r),
            RuntimeExpectation::ExecutedTested { .. }
        ));
    }

    #[test]
    fn zero_shot_mlx_rows_follow_tier_size_evidence() {
        let zs = |repo: &str, tier: &str| {
            let tier = if tier.is_empty() {
                "null".to_owned()
            } else {
                format!("\"{tier}\"")
            };
            let line = format!(
                r#"{{"repo_id":"{repo}","family":"ZeroShot","task":"token-classification","tier":{tier},"architecture":"deberta-v2","formats":["mlx-fp","pytorch"],"license":"apache-2.0"}}"#
            );
            let catalog = ModelCatalog::import("s/f", COMMIT, line.as_bytes()).unwrap();
            runtime_expectation(catalog.get(repo).unwrap())
        };
        assert_eq!(
            zs(
                "OpenMed/OpenMed-ZeroShot-NER-Pharma-Small-166M-mlx",
                "Small"
            ),
            RuntimeExpectation::ExpectedRunnableZeroShot
        );
        assert_eq!(
            zs(
                "OpenMed/OpenMed-ZeroShot-NER-Pharma-Medium-209M-mlx",
                "Medium"
            ),
            RuntimeExpectation::ExpectedRunnableZeroShot
        );
        for tier in ["Large", ""] {
            assert!(matches!(
                zs("OpenMed/OpenMed-ZeroShot-NER-Pharma-X-mlx", tier),
                RuntimeExpectation::KnownUnsupported { .. }
            ));
        }
        assert!(matches!(
            zs(
                "OpenMed/OpenMed-ZeroShot-NER-Disease-Medium-209M-mlx",
                "Medium"
            ),
            RuntimeExpectation::ExecutedTested { .. }
        ));
    }

    #[test]
    fn rows_too_large_for_any_half_precision_export_are_estimated_over_bound() {
        let sized = |params: u64| {
            let line = format!(
                r#"{{"repo_id":"Org/big","family":"PII","task":"token-classification","architecture":"xlm-roberta","formats":["onnx"],"license":"apache-2.0","param_count":{params}}}"#
            );
            let catalog = ModelCatalog::import("s/f", COMMIT, line.as_bytes()).unwrap();
            runtime_expectation(catalog.get("Org/big").unwrap())
        };
        assert_eq!(sized(278_000_000), RuntimeExpectation::ExpectedRunnable);
        assert_eq!(
            sized(560_000_000),
            RuntimeExpectation::EstimatedOverBound {
                estimated_megabytes: 1_120
            }
        );
        // A recorded run overrides the estimate (QwenMed 600M int8 fits).
        let qwen = named(
            "OpenMed/OpenMed-PII-Chinese-QwenMed-XLarge-600M-v1-onnx-android",
            "token-classification",
            "qwen",
            r#"["onnx"]"#,
        );
        assert!(matches!(
            runtime_expectation(&qwen),
            RuntimeExpectation::ExecutedTested { .. }
        ));
    }

    #[test]
    fn degenerate_tokenizers_are_listed_by_exact_repository() {
        let row = |repo: &str, params: u64| {
            let line = format!(
                r#"{{"repo_id":"{repo}","family":"NER","task":"token-classification","architecture":"xlm-roberta","formats":["onnx"],"license":"apache-2.0","param_count":{params}}}"#
            );
            let catalog = ModelCatalog::import("s/f", COMMIT, line.as_bytes()).unwrap();
            runtime_expectation(catalog.get(repo).unwrap())
        };
        assert_eq!(
            row(
                "OpenMed/OpenMed-PII-BigMed-Large-278M-v1-onnx-android",
                278_000_000
            ),
            RuntimeExpectation::KnownUnsupported {
                reason: DEGENERATE_TOKENIZER_REASON.to_owned()
            },
            "a PII export with a degenerate tokenizer (batch 01)"
        );
        // A well-formed (Unigram) NER export is not blocked by the tokenizer
        // rule; this one is too large for the bound instead.
        assert!(matches!(
            row(
                "OpenMed/OpenMed-NER-AnatomyDetect-MultiMed-568M-v1-onnx-android",
                568_000_000
            ),
            RuntimeExpectation::EstimatedOverBound { .. }
        ));
        assert_eq!(KNOWN_DEFECTIVE_EXPORTS.len(), 28);
    }

    #[test]
    fn executed_rows_report_their_own_evidence_first() {
        let r = named(
            "OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-33M-mlx",
            "token-classification",
            "bert",
            r#"["mlx-fp","pytorch"]"#,
        );
        match runtime_expectation(&r) {
            RuntimeExpectation::ExecutedTested {
                runtime,
                revision,
                evidence_run,
                ..
            } => {
                assert_eq!(runtime, "candle_mlx");
                assert_eq!(revision.len(), 40);
                assert_eq!(evidence_run, "38039625442");
            }
            other => panic!("{other:?}"),
        }
        let mut repos: Vec<&str> = EXECUTED_MODELS.iter().map(|m| m.repo_id).collect();
        repos.sort_unstable();
        repos.dedup();
        assert_eq!(repos.len(), EXECUTED_MODELS.len(), "one entry per row");
        for m in EXECUTED_MODELS {
            assert_eq!(m.revision.len(), 40, "{}", m.repo_id);
            assert!(m.revision.bytes().all(|b| b.is_ascii_hexdigit()));
            assert!(m.evidence_run.bytes().all(|b| b.is_ascii_digit()));
        }
    }

    #[test]
    fn expectation_never_changes_the_row_status() {
        let r = row("token-classification", "bert", r#"["onnx"]"#);
        let before = r.status;
        let _ = runtime_expectation(&r);
        assert_eq!(r.status, before);
        assert!(before <= crate::catalog::CatalogStatus::RightsPending);
    }
}
