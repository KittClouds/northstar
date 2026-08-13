use northstar_market_objects::{RawCorpus, normalize_expansion_samples, pack_raw_corpus};
use serde::Serialize;
use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
};

fn write_json(path: PathBuf, value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let mut writer = BufWriter::with_capacity(64 * 1024, file);
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let root = PathBuf::from(
        args.next()
            .ok_or("usage: northstar-market-object-seal ROOT STEM OUTPUT")?,
    );
    let stem = args
        .next()
        .ok_or("missing STEM")?
        .into_string()
        .map_err(|_| "STEM is not UTF-8")?;
    let output = PathBuf::from(args.next().ok_or("missing OUTPUT")?);
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    fs::create_dir_all(&output)?;
    let corpus = RawCorpus::open(&root, &stem)?;
    let packed_path = output.join(format!("{}.nsmor", corpus.report().run_key));
    let pack = pack_raw_corpus(&corpus, packed_path)?;
    let (_, derived) = normalize_expansion_samples(&corpus)?;
    write_json(output.join("raw-corpus-receipt.json"), corpus.report())?;
    write_json(output.join("packed-corpus-receipt.json"), &pack)?;
    write_json(output.join("derived-recipe-receipt.json"), &derived)?;
    println!(
        "RG3_SEALED run_key={} raw_sha256={} packed_sha256={}",
        corpus.report().run_key,
        corpus.report().canonical_sha256,
        pack.packed_sha256
    );
    Ok(())
}
