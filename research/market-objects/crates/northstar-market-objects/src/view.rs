use crate::{RawCorpus, RawError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const RECIPE: &str = "NORTHSTAR_RG3_DERIVED_ORIGIN_ATR_NORMALIZATION_V1";

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedExpansionSample {
    pub expansion_id: i64,
    pub bar_time: i64,
    pub displacement_origin_atr: f64,
    pub velocity_origin_atr_per_bar: f64,
    pub return_ratio: Option<f64>,
    pub path_efficiency: Option<f64>,
    pub oriented_efficiency: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedReceipt {
    pub contract: String,
    pub raw_corpus_sha256: String,
    pub recipe_id: String,
    pub recipe_sha256: String,
    pub row_count: usize,
}

pub fn normalize_expansion_samples(
    corpus: &RawCorpus,
) -> Result<(Vec<NormalizedExpansionSample>, DerivedReceipt), RawError> {
    let mut output = Vec::with_capacity(corpus.report().rows["expansion_samples"]);
    for row in corpus.rows("expansion_samples") {
        let parse = |name: &str| -> Result<f64, RawError> {
            row.field(name)?.parse().map_err(|_| RawError::Invalid {
                dataset: "expansion_samples".into(),
                detail: format!("field {name} is not f64"),
            })
        };
        let atr = parse("origin_atr")?;
        if atr <= f64::EPSILON {
            continue;
        }
        let displacement = parse("raw_displacement")?;
        let max_displacement = parse("raw_max_displacement")?;
        let return_depth = parse("raw_return_depth")?;
        let path = parse("raw_close_path_length")?;
        output.push(NormalizedExpansionSample {
            expansion_id: row.i64("expansion_id")?,
            bar_time: row.i64("bar_time")?,
            displacement_origin_atr: displacement / atr,
            velocity_origin_atr_per_bar: parse("raw_velocity_per_bar")? / atr,
            return_ratio: (max_displacement > f64::EPSILON)
                .then_some(return_depth / max_displacement),
            path_efficiency: (path > f64::EPSILON).then_some(displacement.abs() / path),
            oriented_efficiency: (path > f64::EPSILON).then_some(displacement / path),
        });
    }
    let receipt = DerivedReceipt {
        contract: "NORTHSTAR_RG3_DERIVED_VIEW_RECEIPT_V1".into(),
        raw_corpus_sha256: corpus.report().canonical_sha256.clone(),
        recipe_id: RECIPE.into(),
        recipe_sha256: hex(Sha256::digest(RECIPE.as_bytes())),
        row_count: output.len(),
    };
    Ok((output, receipt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::write_fixture;
    #[test]
    fn derived_view_is_receipted_and_does_not_mutate_raw() {
        let dir = tempfile::tempdir().unwrap();
        write_fixture(dir.path(), "fixture");
        let corpus = RawCorpus::open(dir.path(), "fixture").unwrap();
        let raw_hash = corpus.report().canonical_sha256.clone();
        let (rows, receipt) = normalize_expansion_samples(&corpus).unwrap();
        assert_eq!(rows[0].displacement_origin_atr, 1.5);
        assert_eq!(rows[0].velocity_origin_atr_per_bar, 0.25);
        assert_eq!(rows[0].return_ratio, Some(0.5 / 3.5));
        assert_eq!(rows[0].path_efficiency, Some(0.75));
        assert_eq!(receipt.raw_corpus_sha256, raw_hash);
        assert_eq!(corpus.report().canonical_sha256, raw_hash);
    }
}
