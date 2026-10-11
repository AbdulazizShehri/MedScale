//! Authority facade, store, promotion, and identity operations.

mod amend;
mod analytics;
mod audio;
mod browse;
mod collaboration;
mod compute;
mod corpus;
mod data_acquire;
mod data_sources;
mod document_ops;
mod durable;
mod extensions;
mod facade;
mod federation;
mod handles;
mod hub;
mod huddles;
mod identity;
mod ingest_ops;
mod institutional;
mod knowledge;
mod medagent;
mod model_fleet;
mod model_fleet_compare;
mod presentation;
mod privacy_gate;
mod privacy_recognizers;
mod project_graph;
mod promote;
mod r_workspace;
mod research_packs;
mod retrieval;
mod source_ops;
mod store;

pub use audio::{
    AsrEngine, FixtureAsrEngine, SegmentEdit, SignalAnalysis, analyze_pcm, build_wav, parse_wav,
    synthetic_pcm,
};
pub use corpus::{
    DEFAULT_CORPUS_ID, DEFAULT_CORPUS_VERSION, SCALE_CORPUS_DOC_COUNT_10K, SCALE_CORPUS_ID,
    SCALE_CORPUS_VERSION_10K, admit_corpus_bytes, admit_corpus_dir,
    build_synthetic_lexical_scale_corpus, default_synthetic_corpus, scale_synthetic_corpus_10k,
};
pub use document_ops::{document_worker_policy, mime_decision, voice_worker_policy};
pub use extensions::{generate_publisher_key, sign_extension_pack, valid_public_key};
pub use facade::CoreFacade;
pub use handles::assert_no_secret_handles;
pub use source_ops::{create_source_record, overwrite_source_bytes, verify_source_digest};
pub use store::InMemoryAuthorityStore;
