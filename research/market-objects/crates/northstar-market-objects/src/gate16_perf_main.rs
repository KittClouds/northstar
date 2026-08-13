use northstar_market_objects::run_gate16_performance_fixture;
use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let usage = "usage: northstar-market-object-geometry-perf PACKED_VECTORS VECTOR_INDEX FIXTURE_OUTPUT REPORT_OUTPUT";
    let packed = PathBuf::from(args.next().ok_or(usage)?);
    let index = PathBuf::from(args.next().ok_or(usage)?);
    let fixture = PathBuf::from(args.next().ok_or(usage)?);
    let report = PathBuf::from(args.next().ok_or(usage)?);
    if fixture.exists() || report.exists() {
        return Err("performance outputs already exist".into());
    }
    let result = run_gate16_performance_fixture(&packed, &index, &fixture)?;
    let mut writer = BufWriter::new(File::create(report)?);
    serde_json::to_writer_pretty(&mut writer, &result)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    println!(
        "GATE16_PERF status={} objects={} dimensions={} scan_us={} parallel_us={} mmap_us={}",
        result.status,
        result.fixture_objects,
        result.dimensions,
        result.scalar_scan_microseconds,
        result.parallel_scan_microseconds,
        result.mmap_traversal_microseconds
    );
    Ok(())
}
