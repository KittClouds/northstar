use crate::gate16_distance::{squared_l2_scalar, squared_l2_simd};
use memmap2::MmapOptions;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
    time::Instant,
};

const FIXTURE_OBJECTS: usize = 100_000;
type PackedSource = (Vec<Vec<f32>>, usize);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate16PerformanceReport {
    pub contract: String,
    pub status: String,
    pub scientific_evidence: bool,
    pub computational_evidence: bool,
    pub fixture_objects: usize,
    pub dimensions: usize,
    pub packed_bytes: usize,
    pub construction_microseconds: u128,
    pub scalar_scan_microseconds: u128,
    pub parallel_scan_microseconds: u128,
    pub top10_selection_microseconds: u128,
    pub mmap_traversal_microseconds: u128,
    pub scalar_simd_max_error: f64,
    pub scalar_distance_sha256: String,
    pub parallel_distance_sha256: String,
    pub mmap_value_sha256: String,
    pub thread_count: usize,
    pub claimed_workload: String,
    pub excluded_claim: String,
}

fn parse_vectors(
    packed_path: &Path,
    index_path: &Path,
) -> Result<PackedSource, Box<dyn std::error::Error>> {
    let file = File::open(packed_path)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    let index = fs::read_to_string(index_path)?;
    let mut vectors = Vec::new();
    let mut dimensions = 0usize;
    for line in index.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 6 || fields[1] != "SUMMARY_CANONICAL_V1" || fields[2] != "AVAILABLE" {
            continue;
        }
        let offset = fields[3].parse::<usize>()?;
        let length = fields[4].parse::<usize>()?;
        dimensions = fields[5].parse::<usize>()?;
        if length != dimensions * 4 || offset + length > mmap.len() {
            return Err("packed vector index out of bounds".into());
        }
        let mut vector = Vec::with_capacity(dimensions);
        for bytes in mmap[offset..offset + length].chunks_exact(4) {
            vector.push(f32::from_le_bytes(bytes.try_into().unwrap()));
        }
        vectors.push(vector);
    }
    if vectors.is_empty() || dimensions == 0 {
        return Err("no canonical summary vectors available".into());
    }
    Ok((vectors, dimensions))
}

fn hash_distances(values: &[f64]) -> String {
    let mut hash = Sha256::new();
    for value in values {
        hash.update(value.to_bits().to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

pub fn run_gate16_performance_fixture(
    packed_path: &Path,
    index_path: &Path,
    fixture_path: &Path,
) -> Result<Gate16PerformanceReport, Box<dyn std::error::Error>> {
    let (source, dimensions) = parse_vectors(packed_path, index_path)?;
    let construction_start = Instant::now();
    let mut fixture = Vec::with_capacity(FIXTURE_OBJECTS * dimensions);
    for index in 0..FIXTURE_OBJECTS {
        fixture.extend_from_slice(&source[index % source.len()]);
    }
    let construction_microseconds = construction_start.elapsed().as_micros();
    let query = fixture[..dimensions].to_vec();
    let scalar_start = Instant::now();
    let scalar = fixture
        .chunks_exact(dimensions)
        .map(|row| squared_l2_simd(&query, row))
        .collect::<Vec<_>>();
    let scalar_scan_microseconds = scalar_start.elapsed().as_micros();
    let parallel_start = Instant::now();
    let parallel = fixture
        .par_chunks_exact(dimensions)
        .map(|row| squared_l2_simd(&query, row))
        .collect::<Vec<_>>();
    let parallel_scan_microseconds = parallel_start.elapsed().as_micros();
    let top_start = Instant::now();
    let mut top = [(f64::INFINITY, usize::MAX); 10];
    for (index, &distance) in scalar.iter().enumerate() {
        if distance < top[9].0 || (distance == top[9].0 && index < top[9].1) {
            top[9] = (distance, index);
            top.sort_by(|left, right| {
                left.0
                    .total_cmp(&right.0)
                    .then_with(|| left.1.cmp(&right.1))
            });
        }
    }
    let top10_selection_microseconds = top_start.elapsed().as_micros();
    let mut maximum_error = 0.0f64;
    for row in fixture.chunks_exact(dimensions).take(1024) {
        maximum_error = maximum_error
            .max((squared_l2_scalar(&query, row) - squared_l2_simd(&query, row)).abs());
    }
    let mut writer = BufWriter::with_capacity(1024 * 1024, File::create(fixture_path)?);
    writer.write_all(b"NSG16PERF1\0")?;
    for value in &fixture {
        writer.write_all(&value.to_le_bytes())?;
    }
    writer.flush()?;
    let mmap_start = Instant::now();
    let fixture_file = File::open(fixture_path)?;
    let mmap = unsafe { MmapOptions::new().map(&fixture_file)? };
    let mut mmap_hash = Sha256::new();
    for chunk in mmap[11..].chunks_exact(4) {
        mmap_hash.update(chunk);
    }
    let mmap_traversal_microseconds = mmap_start.elapsed().as_micros();
    let scalar_sha = hash_distances(&scalar);
    let parallel_sha = hash_distances(&parallel);
    let status = if scalar_sha == parallel_sha && maximum_error <= 1e-3 {
        "PASS"
    } else {
        "FAIL"
    };
    Ok(Gate16PerformanceReport {
        contract: "NORTHSTAR_RG3_GATE16_PERFORMANCE_FIXTURE_V1".into(), status: status.into(), scientific_evidence: false, computational_evidence: true,
        fixture_objects: FIXTURE_OBJECTS, dimensions, packed_bytes: fixture.len() * 4,
        construction_microseconds, scalar_scan_microseconds, parallel_scan_microseconds, top10_selection_microseconds, mmap_traversal_microseconds,
        scalar_simd_max_error: maximum_error, scalar_distance_sha256: scalar_sha, parallel_distance_sha256: parallel_sha,
        mmap_value_sha256: format!("{:x}", mmap_hash.finalize()), thread_count: rayon::current_num_threads(),
        claimed_workload: "representation construction; one-query to 100k scan; deterministic top-10 selection; mmap traversal; scalar/SIMD and parallel parity".into(),
        excluded_claim: "100k by 100k all-pairs matrix construction was not run or claimed".into(),
    })
}
