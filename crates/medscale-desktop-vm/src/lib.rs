//! Desktop workspace view-models, shared by the Slint reference application and
//! the Tauri desktop.
//!
//! The module sources stay in `crates/medscale-desktop/src/` because Core
//! qualification tests read them by path. This crate compiles them in place
//! (no copies), so both shells map the same Core contracts the same way. Every
//! function here takes a Rust-owned `CliSession`; nothing reaches a frontend
//! except the typed rows these functions return.

#![allow(dead_code)]

#[path = "../../medscale-desktop/src/analytics_workspace.rs"]
pub mod analytics_workspace;
#[path = "../../medscale-desktop/src/audio_workspace.rs"]
pub mod audio_workspace;
#[path = "../../medscale-desktop/src/browse_workspace.rs"]
pub mod browse_workspace;
#[path = "../../medscale-desktop/src/collaboration_workspace.rs"]
pub mod collaboration_workspace;
#[path = "../../medscale-desktop/src/data_workbench.rs"]
pub mod data_workbench;
#[path = "../../medscale-desktop/src/knowledge_workspace.rs"]
pub mod knowledge_workspace;
#[path = "../../medscale-desktop/src/medagent_workspace.rs"]
pub mod medagent_workspace;
#[path = "../../medscale-desktop/src/model_fleet_workspace.rs"]
pub mod model_fleet_workspace;
#[path = "../../medscale-desktop/src/patient_workspace.rs"]
pub mod patient_workspace;
#[path = "../../medscale-desktop/src/population_insights.rs"]
pub mod population_insights;
#[path = "../../medscale-desktop/src/privacy_workspace.rs"]
pub mod privacy_workspace;
#[path = "../../medscale-desktop/src/product_intelligence.rs"]
pub mod product_intelligence;
#[path = "../../medscale-desktop/src/project_workspace.rs"]
pub mod project_workspace;
#[path = "../../medscale-desktop/src/research_os_workspace.rs"]
pub mod research_os_workspace;
#[path = "../../medscale-desktop/src/utility_surfaces.rs"]
pub mod utility_surfaces;
#[path = "../../medscale-desktop/src/workflow_studio.rs"]
pub mod workflow_studio;
