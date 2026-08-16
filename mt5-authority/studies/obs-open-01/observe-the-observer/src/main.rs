mod census;
mod seal;
mod source;
mod topology;

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str) {
        Some("build") if args.len() == 4 => {
            println!(
                "{}",
                seal::build(&PathBuf::from(&args[2]), &PathBuf::from(&args[3]))?
            )
        }
        Some("verify") if args.len() == 3 => {
            println!("{}", seal::verify(&PathBuf::from(&args[2]))?)
        }
        Some("finalize") if args.len() == 5 => println!(
            "{}",
            seal::finalize(
                &PathBuf::from(&args[2]),
                &PathBuf::from(&args[3]),
                &PathBuf::from(&args[4])
            )?
        ),
        Some("topology-build") if args.len() == 4 => println!(
            "{}",
            topology::build(&PathBuf::from(&args[2]), &PathBuf::from(&args[3]))?
        ),
        Some("topology-verify") if args.len() == 3 => {
            println!("{}", topology::verify(&PathBuf::from(&args[2]))?)
        }
        Some("topology-finalize") if args.len() == 5 => println!(
            "{}",
            topology::finalize(
                &PathBuf::from(&args[2]),
                &PathBuf::from(&args[3]),
                &PathBuf::from(&args[4])
            )?
        ),
        _ => {
            return Err(
                "usage: obs-open-observe-the-observer <build REPO OUT|verify SEAL|finalize A B SEAL|topology-build REPO OUT|topology-verify SEAL|topology-finalize A B SEAL>"
                    .into(),
            );
        }
    }
    Ok(())
}
