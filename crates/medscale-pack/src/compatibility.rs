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
