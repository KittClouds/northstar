use super::{FamilySupport, MIN_FAMILY_SUPPORT, ObjectRecord, RECIPE, ViewReport};
use hashbrown::{HashMap, HashSet};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use wide::f32x8;

struct Matrix {
    rows: usize,
    cols: usize,
    data: Vec<f32>,
}

impl Matrix {
    fn row(&self, row: usize) -> &[f32] {
        &self.data[row * self.cols..(row + 1) * self.cols]
    }
}

fn feature<'a>(record: &'a ObjectRecord, view: &str) -> &'a [f32] {
    match view {
        "summary_geometry_v1" => &record.summary,
        "resampled_shape_v1" => &record.shape,
        _ => &record.hybrid,
    }
}

struct Standardized {
    matrix: Matrix,
    lows: Vec<f32>,
    highs: Vec<f32>,
    means: Vec<f64>,
    scales: Vec<f64>,
}

fn standardized(records: &[&ObjectRecord], view: &str) -> Standardized {
    let cols = feature(records[0], view).len();
    let mut lows = vec![0.0f32; cols];
    let mut highs = vec![0.0f32; cols];
    let mut column = Vec::with_capacity(records.len());
    for feature_index in 0..cols {
        column.clear();
        column.extend(
            records
                .iter()
                .map(|record| feature(record, view)[feature_index]),
        );
        column.sort_by(f32::total_cmp);
        lows[feature_index] = column[(column.len() - 1) / 100];
        highs[feature_index] = column[((column.len() - 1) * 99) / 100];
    }
    let mut means = vec![0.0f64; cols];
    for record in records {
        for (((mean, &value), &low), &high) in means
            .iter_mut()
            .zip(feature(record, view))
            .zip(&lows)
            .zip(&highs)
        {
            *mean += value.clamp(low, high) as f64;
        }
    }
    for mean in &mut means {
        *mean /= records.len() as f64;
    }
    let mut scales = vec![0.0f64; cols];
    for record in records {
        for ((((scale, &value), mean), &low), &high) in scales
            .iter_mut()
            .zip(feature(record, view))
            .zip(&means)
            .zip(&lows)
            .zip(&highs)
        {
            *scale += (value.clamp(low, high) as f64 - mean).powi(2);
        }
    }
    for scale in &mut scales {
        *scale = (*scale / records.len() as f64).sqrt().max(1e-9);
    }
    let mut data = Vec::with_capacity(records.len() * cols);
    for record in records {
        for ((((&value, mean), scale), &low), &high) in feature(record, view)
            .iter()
            .zip(&means)
            .zip(&scales)
            .zip(&lows)
            .zip(&highs)
        {
            data.push(((value.clamp(low, high) as f64 - mean) / scale).clamp(-6.0, 6.0) as f32);
        }
    }
    Standardized {
        matrix: Matrix {
            rows: records.len(),
            cols,
            data,
        },
        lows,
        highs,
        means,
        scales,
    }
}

pub(super) fn squared_distance(left: &[f32], right: &[f32]) -> f32 {
    let mut total = 0.0;
    let packed = left.len() / 8 * 8;
    for offset in (0..packed).step_by(8) {
        let a = f32x8::from(&left[offset..offset + 8]);
        let b = f32x8::from(&right[offset..offset + 8]);
        let delta = a - b;
        total += (delta * delta).reduce_add();
    }
    for index in packed..left.len() {
        let delta = left[index] - right[index];
        total += delta * delta;
    }
    total
}

struct KmeansResult {
    labels: Vec<usize>,
    centroids: Vec<f32>,
}

fn kmeans(matrix: &Matrix, k: usize, seed: usize) -> KmeansResult {
    let mut centroids = Vec::with_capacity(k * matrix.cols);
    let first = seed.wrapping_mul(104_729).wrapping_add(17) % matrix.rows;
    centroids.extend_from_slice(matrix.row(first));
    let mut nearest = vec![f32::MAX; matrix.rows];
    for _ in 1..k {
        let latest = &centroids[centroids.len() - matrix.cols..];
        for (row, distance) in nearest.iter_mut().enumerate() {
            *distance = distance.min(squared_distance(matrix.row(row), latest));
        }
        let next = nearest
            .iter()
            .enumerate()
            .max_by(|(li, lv), (ri, rv)| lv.total_cmp(rv).then_with(|| ri.cmp(li)))
            .map_or(0, |(index, _)| index);
        centroids.extend_from_slice(matrix.row(next));
    }
    let mut labels = vec![usize::MAX; matrix.rows];
    for _ in 0..100 {
        let mut changed = false;
        for (row, label) in labels.iter_mut().enumerate() {
            let next = (0..k)
                .min_by(|&left, &right| {
                    squared_distance(
                        matrix.row(row),
                        &centroids[left * matrix.cols..(left + 1) * matrix.cols],
                    )
                    .total_cmp(&squared_distance(
                        matrix.row(row),
                        &centroids[right * matrix.cols..(right + 1) * matrix.cols],
                    ))
                })
                .unwrap_or(0);
            changed |= *label != next;
            *label = next;
        }
        let mut sums = vec![0.0f32; k * matrix.cols];
        let mut counts = vec![0usize; k];
        for (row, &label) in labels.iter().enumerate() {
            counts[label] += 1;
            for (sum, &value) in sums[label * matrix.cols..(label + 1) * matrix.cols]
                .iter_mut()
                .zip(matrix.row(row))
            {
                *sum += value;
            }
        }
        for cluster in 0..k {
            if counts[cluster] == 0 {
                let replacement = (seed + cluster * 7_919) % matrix.rows;
                sums[cluster * matrix.cols..(cluster + 1) * matrix.cols]
                    .copy_from_slice(matrix.row(replacement));
                counts[cluster] = 1;
            }
            for value in &mut sums[cluster * matrix.cols..(cluster + 1) * matrix.cols] {
                *value /= counts[cluster] as f32;
            }
        }
        centroids = sums;
        if !changed {
            break;
        }
    }
    KmeansResult { labels, centroids }
}

fn choose_two(value: usize) -> f64 {
    value.saturating_mul(value.saturating_sub(1)) as f64 / 2.0
}

pub(super) fn adjusted_rand(left: &[usize], right: &[usize]) -> f64 {
    let mut cells = HashMap::new();
    let mut left_counts = HashMap::new();
    let mut right_counts = HashMap::new();
    for (&a, &b) in left.iter().zip(right) {
        *cells.entry((a, b)).or_insert(0usize) += 1;
        *left_counts.entry(a).or_insert(0usize) += 1;
        *right_counts.entry(b).or_insert(0usize) += 1;
    }
    let pairs = choose_two(left.len());
    if pairs == 0.0 {
        return 1.0;
    }
    let index: f64 = cells.values().map(|&count| choose_two(count)).sum();
    let left_sum: f64 = left_counts.values().map(|&count| choose_two(count)).sum();
    let right_sum: f64 = right_counts.values().map(|&count| choose_two(count)).sum();
    let expected = left_sum * right_sum / pairs;
    let maximum = 0.5 * (left_sum + right_sum);
    if (maximum - expected).abs() < f64::EPSILON {
        1.0
    } else {
        (index - expected) / (maximum - expected)
    }
}

fn centroid_silhouette(matrix: &Matrix, result: &KmeansResult, k: usize) -> f64 {
    let mut total = 0.0;
    for row in 0..matrix.rows {
        let own = result.labels[row];
        let a = squared_distance(
            matrix.row(row),
            &result.centroids[own * matrix.cols..(own + 1) * matrix.cols],
        )
        .sqrt();
        let b = (0..k)
            .filter(|&cluster| cluster != own)
            .map(|cluster| {
                squared_distance(
                    matrix.row(row),
                    &result.centroids[cluster * matrix.cols..(cluster + 1) * matrix.cols],
                )
                .sqrt()
            })
            .fold(f32::INFINITY, f32::min);
        total += if a.max(b) > f32::EPSILON {
            ((b - a) / a.max(b)) as f64
        } else {
            0.0
        };
    }
    total / matrix.rows as f64
}

pub(super) struct ViewAnalysis {
    pub(super) report: ViewReport,
    pub(super) labels: Vec<usize>,
    pub(super) winsor_low: Vec<f32>,
    pub(super) winsor_high: Vec<f32>,
    pub(super) means: Vec<f64>,
    pub(super) scales: Vec<f64>,
    pub(super) centroids: Vec<f32>,
}

pub(super) fn analyze_view(records: &[&ObjectRecord], view: &str) -> ViewAnalysis {
    let standardized = standardized(records, view);
    let matrix = &standardized.matrix;
    let max_k = 8usize.min((matrix.rows / 10).max(2));
    let mut best: Option<(f64, f64, f64, KmeansResult)> = None;
    for k in 2..=max_k {
        let base = kmeans(matrix, k, 0);
        let stability = (1..8)
            .map(|seed| adjusted_rand(&base.labels, &kmeans(matrix, k, seed).labels))
            .sum::<f64>()
            / 7.0;
        let silhouette = centroid_silhouette(matrix, &base, k);
        let mut counts = vec![0usize; k];
        for &label in &base.labels {
            counts[label] += 1;
        }
        let supported: Vec<_> = counts
            .iter()
            .copied()
            .filter(|&count| count >= MIN_FAMILY_SUPPORT)
            .collect();
        let support = if supported.len() >= 2 {
            let covered: usize = supported.iter().sum();
            let smallest = *supported.iter().min().unwrap_or(&0) as f64;
            let largest = *supported.iter().max().unwrap_or(&1) as f64;
            covered as f64 / matrix.rows as f64 * (smallest / largest)
        } else {
            0.0
        };
        let score = silhouette.max(0.0) * stability.max(0.0) * support;
        if best.as_ref().is_none_or(|current| score > current.0) {
            best = Some((score, silhouette, stability, base));
        }
    }
    let (score, silhouette, stability, result) = best.expect("at least k=2");
    let k = result.centroids.len() / matrix.cols;
    let mut families = Vec::with_capacity(k);
    for cluster in 0..k {
        let members: Vec<_> = result
            .labels
            .iter()
            .enumerate()
            .filter(|&(_, label)| *label == cluster)
            .map(|(index, _)| index)
            .collect();
        let mut instrument_counts = BTreeMap::new();
        for &index in &members {
            *instrument_counts
                .entry(records[index].instrument.clone())
                .or_insert(0) += 1;
        }
        let runs: HashSet<_> = members
            .iter()
            .map(|&index| records[index].run_key.clone())
            .collect();
        let mut window_counts = BTreeMap::new();
        for &index in &members {
            let run = &records[index].run_key;
            let mut parts = run.rsplit('_');
            let _hash = parts.next();
            let end = parts.next().unwrap_or("");
            let start = parts.next().unwrap_or("");
            *window_counts.entry(format!("{start}:{end}")).or_insert(0) += 1;
        }
        let medoid = members
            .iter()
            .min_by(|&&left, &&right| {
                squared_distance(
                    matrix.row(left),
                    &result.centroids[cluster * matrix.cols..(cluster + 1) * matrix.cols],
                )
                .total_cmp(&squared_distance(
                    matrix.row(right),
                    &result.centroids[cluster * matrix.cols..(cluster + 1) * matrix.cols],
                ))
            })
            .copied()
            .unwrap_or(0);
        let instrument_names: Vec<_> = instrument_counts.keys().cloned().collect();
        let largest_instrument_fraction = instrument_counts.values().copied().max().unwrap_or(0)
            as f64
            / members.len().max(1) as f64;
        let largest_window_fraction =
            window_counts.values().copied().max().unwrap_or(0) as f64 / members.len().max(1) as f64;
        let common_support = members.len() >= MIN_FAMILY_SUPPORT
            && instrument_names.len() >= 4
            && window_counts.len() >= 4;
        families.push(FamilySupport {
            candidate_id: format!("C{}", cluster + 1),
            count: members.len(),
            fraction: members.len() as f64 / records.len() as f64,
            instrument_count: instrument_names.len(),
            instrument_counts,
            largest_instrument_fraction,
            window_count: window_counts.len(),
            window_counts,
            largest_window_fraction,
            run_count: runs.len(),
            common_support,
            instruments: instrument_names,
            medoid_run_key: records[medoid].run_key.clone(),
            medoid_object_id: records[medoid].object_id,
        });
    }
    let minimum_cluster_support = families
        .iter()
        .map(|family| family.count)
        .min()
        .unwrap_or(0);
    let supported_objects = families
        .iter()
        .filter(|family| family.common_support)
        .map(|family| family.count)
        .sum();
    let noise_objects = records.len().saturating_sub(supported_objects);
    let mut identity_rows: Vec<_> = records
        .iter()
        .zip(&result.labels)
        .map(|(record, &label)| {
            let candidate = if families[label].common_support {
                format!("C{}", label + 1)
            } else {
                "NULL".into()
            };
            format!(
                "{}\t{}\t{}\t{}\t{}",
                record.kind.name(),
                record.run_key,
                record.object_id,
                view,
                candidate
            )
        })
        .collect();
    identity_rows.sort();
    let family_system_sha256 = format!(
        "{:x}",
        Sha256::digest(format!("{RECIPE}\n{}\n", identity_rows.join("\n")).as_bytes())
    );
    ViewAnalysis {
        report: ViewReport {
            representation: view.to_owned(),
            family_system_sha256,
            dimensions: matrix.cols,
            selected_k: k,
            selection_score: score,
            centroid_silhouette: silhouette,
            mean_seed_ari: stability,
            minimum_cluster_support,
            supported_objects,
            noise_objects,
            noise_fraction: noise_objects as f64 / records.len().max(1) as f64,
            families,
        },
        labels: result.labels,
        winsor_low: standardized.lows,
        winsor_high: standardized.highs,
        means: standardized.means,
        scales: standardized.scales,
        centroids: result.centroids,
    }
}
