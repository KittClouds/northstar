mod authority_impact;
mod authority_surface;
mod census;
mod covariation;
mod intervention;
mod kammi_campaign;
mod kammi_p2;
mod literature;
mod runtime_transport;
mod seal;
mod sol_campaign;
mod sol_part2;
mod sol_part2b;
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
        Some("authority-surface-build") if args.len() == 4 => println!(
            "{}",
            authority_surface::build(&PathBuf::from(&args[2]), &PathBuf::from(&args[3]))?
        ),
        Some("authority-surface-verify") if args.len() == 3 => {
            println!("{}", authority_surface::verify(&PathBuf::from(&args[2]))?)
        }
        Some("authority-surface-finalize") if args.len() == 5 => println!(
            "{}",
            authority_surface::finalize(
                &PathBuf::from(&args[2]),
                &PathBuf::from(&args[3]),
                &PathBuf::from(&args[4])
            )?
        ),
        Some("literature-build") if args.len() == 4 => println!(
            "{}",
            literature::build(&PathBuf::from(&args[2]), &PathBuf::from(&args[3]))?
        ),
        Some("literature-verify") if args.len() == 3 => {
            println!("{}", literature::verify(&PathBuf::from(&args[2]))?)
        }
        Some("literature-finalize") if args.len() == 5 => println!(
            "{}",
            literature::finalize(
                &PathBuf::from(&args[2]),
                &PathBuf::from(&args[3]),
                &PathBuf::from(&args[4])
            )?
        ),
        Some("sol-build") if args.len() == 4 => println!(
            "{}",
            sol_campaign::build(&PathBuf::from(&args[2]), &PathBuf::from(&args[3]))?
        ),
        Some("sol-verify") if args.len() == 3 => {
            println!("{}", sol_campaign::verify(&PathBuf::from(&args[2]))?)
        }
        Some("sol-finalize") if args.len() == 5 => println!(
            "{}",
            sol_campaign::finalize(
                &PathBuf::from(&args[2]),
                &PathBuf::from(&args[3]),
                &PathBuf::from(&args[4])
            )?
        ),
        Some("sol-part2-build") if args.len() == 4 => println!(
            "{}",
            sol_part2::build(&PathBuf::from(&args[2]), &PathBuf::from(&args[3]))?
        ),
        Some("sol-part2-verify") if args.len() == 3 => {
            println!("{}", sol_part2::verify(&PathBuf::from(&args[2]))?)
        }
        Some("sol-part2-finalize") if args.len() == 5 => println!(
            "{}",
            sol_part2::finalize(
                &PathBuf::from(&args[2]),
                &PathBuf::from(&args[3]),
                &PathBuf::from(&args[4])
            )?
        ),
        Some("sol-p2b-build") if args.len() == 4 => println!(
            "{}",
            sol_part2b::build(&PathBuf::from(&args[2]), &PathBuf::from(&args[3]))?
        ),
        Some("sol-p2b-verify") if args.len() == 3 => {
            println!("{}", sol_part2b::verify(&PathBuf::from(&args[2]))?)
        }
        Some("sol-p2b-finalize") if args.len() == 5 => println!(
            "{}",
            sol_part2b::finalize(
                &PathBuf::from(&args[2]),
                &PathBuf::from(&args[3]),
                &PathBuf::from(&args[4])
            )?
        ),
        Some("kammi-build") if args.len() == 4 => println!(
            "{}",
            kammi_campaign::build(&PathBuf::from(&args[2]), &PathBuf::from(&args[3]))?
        ),
        Some("kammi-verify") if args.len() == 3 => {
            println!("{}", kammi_campaign::verify(&PathBuf::from(&args[2]))?)
        }
        Some("kammi-finalize") if args.len() == 5 => println!(
            "{}",
            kammi_campaign::finalize(
                &PathBuf::from(&args[2]),
                &PathBuf::from(&args[3]),
                &PathBuf::from(&args[4])
            )?
        ),
        Some("kammi-p2-build") if args.len() == 4 => println!(
            "{}",
            kammi_p2::build(&PathBuf::from(&args[2]), &PathBuf::from(&args[3]))?
        ),
        Some("kammi-p2-verify") if args.len() == 3 => {
            println!("{}", kammi_p2::verify(&PathBuf::from(&args[2]))?)
        }
        Some("kammi-p2-finalize") if args.len() == 5 => println!(
            "{}",
            kammi_p2::finalize(
                &PathBuf::from(&args[2]),
                &PathBuf::from(&args[3]),
                &PathBuf::from(&args[4])
            )?
        ),
        _ => {
            return Err(
                "usage: obs-open-observe-the-observer <build REPO OUT|verify SEAL|finalize A B SEAL|topology-build REPO OUT|topology-verify SEAL|topology-finalize A B SEAL|rt-build REPO OUT|rt-verify SEAL|rt-finalize A B SEAL|intervention-build REPO OUT|intervention-verify SEAL|intervention-finalize A B SEAL|covariation-build REPO OUT|covariation-verify SEAL|covariation-finalize A B SEAL|authority-impact-build REPO OUT|authority-impact-verify SEAL|authority-impact-finalize A B SEAL|authority-surface-build REPO OUT|authority-surface-verify SEAL|authority-surface-finalize A B SEAL|literature-build REPO OUT|literature-verify SEAL|literature-finalize A B SEAL|sol-build REPO OUT|sol-verify SEAL|sol-finalize A B SEAL|sol-part2-build REPO OUT|sol-part2-verify SEAL|sol-part2-finalize A B SEAL|sol-p2b-build REPO OUT|sol-p2b-verify SEAL|sol-p2b-finalize A B SEAL|kammi-build REPO OUT|kammi-verify SEAL|kammi-finalize A B SEAL|kammi-p2-build REPO OUT|kammi-p2-verify SEAL|kammi-p2-finalize A B SEAL|...>"
                    .into(),
            );
        }
    }
    Ok(())
}
