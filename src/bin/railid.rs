//! Small command-line helper for RailID.

use std::env;
use std::process::ExitCode;

use railid::{decode, generate, generate_many_strings, generate_os_only, RailId};

const USAGE: &str = "\
railid - RailID v1 generator and decoder

Usage:
  railid generate [--degraded]   Generate an ID (TPM hybrid by default)
  railid generate-many <count> [--degraded]
                                 Generate a batch (OS-only uses a supervisor)
  railid decode <id>             Validate and print a canonical ID
  railid inspect <id>            Show timestamp and random fields
  railid --version               Show the RailID version
  railid --help                  Show this help

--degraded uses OS randomness only and is NOT the conforming TPM profile.
";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = run(&args);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(exit) => {
            eprintln!("{exit}");
            ExitCode::from(2)
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("generate" | "gen") => {
            let mut degraded = false;
            for argument in &args[1..] {
                match argument.as_str() {
                    "--degraded" | "--os-only" => degraded = true,
                    other => return Err(format!("unknown option: {other}")),
                }
            }

            let id = if degraded {
                eprintln!("warning: degraded profile: OS CSPRNG only, TPM not used");
                generate_os_only()
            } else {
                generate()
            }
            .map_err(|error| format!("generation failed: {error}"))?;

            println!("{id}");
            Ok(())
        }
        Some("generate-many") => {
            let count_text = args
                .get(1)
                .ok_or("usage: railid generate-many <count> [--degraded]")?;
            let count: usize = count_text
                .parse()
                .map_err(|_| format!("invalid count: {count_text}"))?;
            let mut degraded = false;
            for argument in &args[2..] {
                match argument.as_str() {
                    "--degraded" | "--os-only" => degraded = true,
                    other => return Err(format!("unknown option: {other}")),
                }
            }

            if degraded {
                eprintln!("warning: degraded profile: OS CSPRNG only, TPM not used");
            }
            let profile = if degraded {
                railid::Profile::OsOnlyDegraded
            } else {
                railid::Profile::TpmHybrid
            };
            let ids = generate_many_strings(count, profile)
                .map_err(|error| format!("generation failed: {error}"))?;
            for id in ids {
                println!("{id}");
            }
            Ok(())
        }
        Some("decode") => {
            let input = args.get(1).ok_or("usage: railid decode <id>")?;
            let id = decode(input).map_err(|error| format!("decode failed: {error}"))?;
            println!("{id}");
            Ok(())
        }
        Some("inspect") => {
            let input = args.get(1).ok_or("usage: railid inspect <id>")?;
            let id = RailId::decode(input).map_err(|error| format!("decode failed: {error}"))?;
            print_inspection(&id);
            Ok(())
        }
        Some("--help" | "-h" | "help") | None => {
            print!("{USAGE}");
            Ok(())
        }
        Some("--version" | "-V") => {
            println!("railid {}", railid::VERSION);
            Ok(())
        }
        Some(other) => Err(format!("unknown command: {other}\n\n{USAGE}")),
    }
}

fn print_inspection(id: &RailId) {
    println!("id:              {id}");
    println!("timestamp_ticks: {}", id.timestamp());
    println!("unix_millis:     {}", id.timestamp_millis());
    println!("random_hex:      {}", hex(&id.random()));
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}
