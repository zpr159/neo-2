pub mod graph_store;
pub mod indexes;
pub mod snapshot;
pub mod compression;
pub mod incremental;
pub mod recovery;

pub use graph_store::GraphStore;
pub use indexes::{GraphIndexes, IndexType, IndexStats};
pub use snapshot::{SnapshotManager, GraphSnapshot, SnapshotConfig};
pub use compression::{GraphCompressor, CompressionConfig, CompressionResult};
pub use incremental::{IncrementalUpdater, DeltaChange, DeltaRecord};
pub use recovery::{RecoveryManager, RecoveryPlan, RecoveryStatus};
