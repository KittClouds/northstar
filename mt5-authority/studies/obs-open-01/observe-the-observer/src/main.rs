mod authority_impact;
mod census;
mod covariation;
mod intervention;
mod runtime_transport;
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
        Some("rt-build") if args.len() == 4 => println!(
            "{}",
            runtime_transport::build(&PathBuf::from(&args[2]), &PathBuf::from(&args[3]))?
        ),
        Some("rt-verify") if args.len() == 3 => {
            println!("{}", runtime_transport::verify(&PathBuf::from(&args[2]))?)
        }
        Some("rt-finalize") if args.len() == 5 => println!(
            "{}",
            runtime_transport::finalize(
                &PathBuf::from(&args[2]),
                &PathBuf::from(&args[3]),
                &PathBuf::from(&args[4])
            )?
        ),
        Some("intervention-build") if args.len() == 4 => println!(
            "{}",
            intervention::build(&PathBuf::from(&args[2]), &PathBuf::from(&args[3]))?
        ),
        Some("intervention-verify") if args.len() == 3 => {
            println!("{}", intervention::verify(&PathBuf::from(&args[2]))?)
        }
        Some("intervention-finalize") if args.len() == 5 => println!(
            "{}",
            intervention::finalize(
                &PathBuf::from(&args[2]),
                &PathBuf::from(&args[3]),
                &PathBuf::from(&args[4])
            )?
        ),
        Some("covariation-build") if args.len() == 4 => println!(
            "{}",
            covariation::build(&PathBuf::from(&args[2]), &PathBuf::from(&args[3]))?
        ),
        Some("covariation-verify") if args.len() == 3 => {
            println!("{}", covariation::verify(&PathBuf::from(&args[2]))?)
        }
        Some("covariation-finalize") if args.len() == 5 => println!(
            "{}",
            covariation::finalize(
                &PathBuf::from(&args[2]),
                &PathBuf::from(&args[3]),
                &PathBuf::from(&args[4])
            )?
        ),
        Some("authority-impact-build") if args.len() == 4 => println!(
            "{}",
            authority_impact::build(&PathBuf::from(&args[2]), &PathBuf::from(&args[3]))?
        ),
        Some("authority-impact-verify") if args.len() == 3 => {
            println!("{}", authority_impact::verify(&PathBuf::from(&args[2]))?)
        }
        Some("authority-impact-finalize") if args.len() == 5 => println!(
            "{}",
            authority_impact::finalize(
                &PathBuf::from(&args[2]),
                &PathBuf::from(&args[3]),
                &PathBuf::from(&args[4])
            )?
        ),
        _ => {
            return Err(
                "usage: obs-open-observe-the-observer <build REPO OUT|verify SEAL|finalize A B SEAL|topology-build REPO OUT|topology-verify SEAL|topology-finalize A B SEAL|rt-build REPO OUT|rt-verify SEAL|rt-finalize A B SEAL|intervention-build REPO OUT|intervention-verify SEAL|intervention-finalize A B SEAL|covariation-build REPO OUT|covariation-verify SEAL|covariation-finalize A B SEAL|authority-impact-build REPO OUT|authority-impact-verify SEAL|authority-impact-finalize A B SEAL>"
                    .into(),
            );
        }
    }
    Ok(())
}
