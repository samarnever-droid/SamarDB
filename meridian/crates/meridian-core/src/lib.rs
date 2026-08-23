//! meridian-core — the MERIDIAN cache engine (HELIOS v5 spec, Phases 0–12).

pub mod cdc;
pub mod chronos;
pub mod deadline;
pub mod delta;
pub mod engine;
pub mod epoch;
pub mod flash;
pub mod hash;
pub mod l0;
pub mod mesh;
pub mod oracle;
pub mod prices;
pub mod shard_count;
pub mod side_planes;
pub mod spectrum;
pub mod types;

pub use engine::{Engine, EngineOptions, EngineStats, SetOpts, SetOutcome, Slo, TtlStatus, ValueRef, PROBE_LIMIT};
pub use oracle::{Dep, OracleIndex};
pub use cdc::{CdcOp, CdcRecord, DegradationLevel, WatermarkTracker, OriginTokenBucket};
pub use delta::{DeltaOp, apply_delta, DifferentialAuditor};
pub use chronos::{ChronosStore, VersionRecord};
pub use prices::{DualAscentEngine, PriceVector};
pub use spectrum::{Approx, FidelityLevel};
pub use deadline::{DeadlineScheduler, DegradeAction};
pub use flash::FlashTier;
pub use mesh::MeshCluster;
pub use types::WAYS;
