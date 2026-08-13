//! RG3 raw market-object verification, lossless packing, and derived views.
//!
//! The raw sensor files are authority. Derived representations bind both the
//! raw corpus digest and an explicit recipe digest.

mod pack;
mod raw;
mod view;

pub use pack::{PackReceipt, PackedCorpus, pack_raw_corpus};
pub use raw::{DATASETS, RawCorpus, RawError, RawReport};
pub use view::{DerivedReceipt, NormalizedExpansionSample, normalize_expansion_samples};
