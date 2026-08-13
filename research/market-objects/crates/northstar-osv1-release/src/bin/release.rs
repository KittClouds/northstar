use northstar_osv1_release::{build_release_pack, rehydrate_release, write_rehydration_receipt};
use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("pack") => {
            let repo = PathBuf::from(args.next().ok_or("missing repository root")?);
            let manifest = PathBuf::from(args.next().ok_or("missing release manifest")?);
            let consumer = PathBuf::from(args.next().ok_or("missing consumer contract")?);
            let output = PathBuf::from(args.next().ok_or("missing pack output")?);
            let receipt = PathBuf::from(args.next().ok_or("missing pack receipt")?);
            if args.next().is_some() { return Err("unexpected pack argument".into()); }
            let result = build_release_pack(repo, manifest, consumer, output, receipt)?;
            println!("OSV1_RELEASE_PACK status={} entries={} bytes={} root={}", result.status, result.entry_count, result.packed_bytes, result.osv1_root_sha256);
        }
        Some("rehydrate") => {
            let pack = PathBuf::from(args.next().ok_or("missing release pack")?);
            let output = PathBuf::from(args.next().ok_or("missing rehydration output")?);
            let receipt = PathBuf::from(args.next().ok_or("missing rehydration receipt")?);
            if args.next().is_some() { return Err("unexpected rehydrate argument".into()); }
            let result = rehydrate_release(pack, output)?;
            write_rehydration_receipt(receipt, &result)?;
            println!("OSV1_REHYDRATE status={} entries={} root={}", result.status, result.entry_count, result.osv1_root_sha256);
        }
        _ => return Err("usage: northstar-osv1-release pack <repo> <manifest> <consumer> <pack> <receipt> | rehydrate <pack> <output> <receipt>".into()),
    }
    Ok(())
}
