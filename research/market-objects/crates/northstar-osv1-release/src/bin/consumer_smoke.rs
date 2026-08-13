use northstar_osv1_release::consumer_smoke;
use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let pack = PathBuf::from(args.next().ok_or("missing release pack")?);
    let receipt = PathBuf::from(args.next().ok_or("missing sidecar receipt")?);
    if args.next().is_some() {
        return Err("unexpected consumer argument".into());
    }
    let result = consumer_smoke(pack, receipt)?;
    println!(
        "OSV1_CONSUMER_SMOKE status={} entries={} typed_states={} mutation={} root={}",
        result.status,
        result.authority_entries_read,
        result.typed_states_verified,
        result.source_mutation_count,
        result.parent_osv1_root_sha256
    );
    Ok(())
}
