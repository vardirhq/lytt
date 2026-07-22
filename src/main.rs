mod net;
mod output;
mod process;
mod project;

use std::env;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use net::list_listeners;
use output::{print_json, print_table};
use process::{kill_process, open_url, process_for_inode, ProcessInfo};
use project::detect_project;

#[derive(Debug, Clone)]
struct PortEntry {
    protocol: String,
    address: String,
    port: u16,
    inode: String,
    process: Option<ProcessInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Table,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    List { format: Format },
    Inspect { port: u16, format: Format },
    Kill { port: u16, force: bool },
    Open { port: u16 },
    Watch { interval_ms: u64 },
    Help,
    Version,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lytt: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    match parse_args(env::args().skip(1))? {
        Command::List { format } => render(None, format),
        Command::Inspect { port, format } => render(Some(port), format),
        Command::Kill { port, force } => kill_port(port, force),
        Command::Open { port } => open_port(port),
        Command::Watch { interval_ms } => watch(interval_ms),
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::Version => {
            println!("lytt {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut args: Vec<String> = args.into_iter().collect();
    let format = if take_flag(&mut args, "--json") {
        Format::Json
    } else {
        Format::Table
    };

    if args.is_empty() {
        return Ok(Command::List { format });
    }

    match args[0].as_str() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "-V" | "--version" | "version" => Ok(Command::Version),
        "watch" => {
            let interval_ms = parse_interval(args.get(1))?;
            Ok(Command::Watch { interval_ms })
        }
        "kill" => {
            let force = take_flag(&mut args, "--force") || take_flag(&mut args, "-9");
            let port = parse_required_port(args.get(1))?;
            Ok(Command::Kill { port, force })
        }
        "open" => {
            let port = parse_required_port(args.get(1))?;
            Ok(Command::Open { port })
        }
        value => {
            let port = parse_port(value)?;
            Ok(Command::Inspect { port, format })
        }
    }
}

fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
    if let Some(index) = args.iter().position(|arg| arg == flag) {
        args.remove(index);
        true
    } else {
        false
    }
}

fn parse_required_port(value: Option<&String>) -> Result<u16, String> {
    let value = value.ok_or_else(|| "missing port".to_string())?;
    parse_port(value)
}

fn parse_port(value: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .map_err(|_| format!("expected a TCP port, got `{value}`"))
}

fn parse_interval(value: Option<&String>) -> Result<u64, String> {
    match value {
        Some(raw) => raw
            .parse::<u64>()
            .map(|seconds| seconds.saturating_mul(1000))
            .map_err(|_| format!("expected watch interval seconds, got `{raw}`")),
        None => Ok(1000),
    }
}

fn render(port_filter: Option<u16>, format: Format) -> Result<(), String> {
    let entries = entries(port_filter)?;

    match format {
        Format::Table => print_table(&entries),
        Format::Json => print_json(&entries),
    }

    Ok(())
}

fn watch(interval_ms: u64) -> Result<(), String> {
    loop {
        print!("\x1B[2J\x1B[H");
        render(None, Format::Table)?;
        thread::sleep(Duration::from_millis(interval_ms));
    }
}

fn kill_port(port: u16, force: bool) -> Result<(), String> {
    let entry = find_entry(port)?;
    let process = entry
        .process
        .ok_or_else(|| format!("port {port} is listening, but no owning process was found"))?;

    kill_process(process.pid, force)?;
    println!(
        "Stopped port {} owned by {} (pid {})",
        port,
        process.display_name(),
        process.pid
    );
    Ok(())
}

fn open_port(port: u16) -> Result<(), String> {
    let entry = find_entry(port)?;
    let host = if entry.address == "0.0.0.0" || entry.address == "::" {
        "localhost".to_string()
    } else {
        entry.address
    };
    let url = format!("http://{host}:{}", entry.port);
    open_url(&url)?;
    println!("Opened {url}");
    Ok(())
}

fn find_entry(port: u16) -> Result<PortEntry, String> {
    entries(Some(port))?
        .into_iter()
        .next()
        .ok_or_else(|| format!("nothing is listening on port {port}"))
}

fn entries(port_filter: Option<u16>) -> Result<Vec<PortEntry>, String> {
    let mut entries = list_listeners()?
        .into_iter()
        .filter(|socket| match port_filter {
            Some(port) => socket.port == port,
            None => true,
        })
        .map(|socket| {
            let process = process_for_inode(&socket.inode).map(|mut process| {
                process.project = detect_project(process.cwd.as_deref());
                process
            });

            PortEntry {
                protocol: socket.protocol,
                address: socket.address,
                port: socket.port,
                inode: socket.inode,
                process,
            }
        })
        .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.port);
    Ok(entries)
}

fn print_help() {
    println!(
        "lytt - project-aware localhost port inspector

Usage:
  lytt                 List listening TCP ports
  lytt <port>          Inspect one port
  lytt --json          Print machine-readable output
  lytt open <port>     Open http://localhost:<port>
  lytt kill <port>     Stop the process listening on a port
  lytt kill <port> -9  Force kill the process
  lytt watch [secs]    Refresh the port list

Examples:
  lytt
  lytt 5173
  lytt open 5173
  lytt kill 3000
"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_list_command() {
        assert_eq!(
            parse_args(Vec::<String>::new()).unwrap(),
            Command::List {
                format: Format::Table
            }
        );
    }

    #[test]
    fn parses_port_inspection() {
        assert_eq!(
            parse_args(["5173".to_string()]).unwrap(),
            Command::Inspect {
                port: 5173,
                format: Format::Table
            }
        );
    }

    #[test]
    fn parses_json_flag_anywhere() {
        assert_eq!(
            parse_args(["--json".to_string(), "3000".to_string()]).unwrap(),
            Command::Inspect {
                port: 3000,
                format: Format::Json
            }
        );
    }

    #[test]
    fn parses_force_kill() {
        assert_eq!(
            parse_args([
                "kill".to_string(),
                "3000".to_string(),
                "--force".to_string()
            ])
            .unwrap(),
            Command::Kill {
                port: 3000,
                force: true
            }
        );
    }
}
