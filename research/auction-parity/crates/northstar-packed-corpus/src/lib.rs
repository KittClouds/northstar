//! Immutable mmap artifact for the seven RG2 auction research relations.
//!
//! The source RG2 corpus hash is provenance. `packed_semantic_sha256` is a
//! separate digest over the canonical relation headers and rows.

mod format;
mod pack;
mod view;

pub use format::{PACKED_CONTRACT, PACKED_FORMAT_VERSION, RelationKind};
pub use pack::{PackReceipt, pack_verified_corpus};
pub use view::{PackedCorpus, PackedError, PackedRow, PackedSection};
