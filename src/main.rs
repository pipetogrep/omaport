mod filter;
mod git;
mod kill;
mod proc_scan;

use std::env;
use std::process::ExitCode;

use proc_scan::{ServerInfo, scan_servers};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        return cmd_list(false, false);
    }

    match args[0].as_str() {
        "list" => {
            let mut include_all = false;
            let mut as_json = false;
            for arg in &args[1..] {
                match arg.as_str() {
                    "--all" | "-a" => include_all = true,
                    "--json" | "-j" => as_json = true,
                    "-h" | "--help" => {
                        print_usage();
                        return ExitCode::SUCCESS;
                    }
                    other => {
                        eprintln!("omaport: unknown option for list: {other}");
                        return ExitCode::FAILURE;
                    }
                }
            }
            cmd_list(include_all, as_json)
        }
        "kill" => {
            let mut pid: Option<u32> = None;
            let mut port: Option<u16> = None;
            let mut force = false;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--port" => {
                        i += 1;
                        let Some(val) = args.get(i) else {
                            eprintln!("omaport: --port requires a value");
                            return ExitCode::FAILURE;
                        };
                        match val.parse::<u16>() {
                            Ok(p) => port = Some(p),
                            Err(_) => {
                                eprintln!("omaport: invalid port: {val}");
                                return ExitCode::FAILURE;
                            }
                        }
                    }
                    "--force" | "-f" => force = true,
                    "-h" | "--help" => {
                        print_usage();
                        return ExitCode::SUCCESS;
                    }
                    raw => match raw.parse::<u32>() {
                        Ok(p) => pid = Some(p),
                        Err(_) => {
                            eprintln!("omaport: invalid PID: {raw}");
                            return ExitCode::FAILURE;
                        }
                    },
                }
                i += 1;
            }
            let Some(target_pid) = pid else {
                eprintln!("omaport: usage: omaport kill <pid> [--port <port>] [--force]");
                return ExitCode::FAILURE;
            };
            match kill::kill_server(target_pid, port, force) {
                Ok(()) => ExitCode::SUCCESS,
                Err(err) => {
                    eprintln!("omaport: {err}");
                    ExitCode::FAILURE
                }
            }
        }
        "-h" | "--help" | "help" => {
            print_usage();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("omaport: unknown command: {other}");
            print_usage();
            ExitCode::FAILURE
        }
    }
}

fn cmd_list(include_all: bool, as_json: bool) -> ExitCode {
    let servers = scan_servers(include_all);
    if as_json {
        println!("{}", servers_to_json(&servers));
    } else if servers.is_empty() {
        println!("No local dev servers running.");
    } else {
        for s in &servers {
            let branch = if s.branch.is_empty() {
                String::new()
            } else {
                format!(" [{}]", s.branch)
            };
            println!(
                ":{:<5} {:<12} PID {:<8} {}{}",
                s.port, s.name, s.pid, s.cwd_short, branch
            );
        }
    }
    ExitCode::SUCCESS
}

fn servers_to_json(servers: &[ServerInfo]) -> String {
    let mut out = String::from("{\"servers\":[");
    for (idx, s) in servers.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str(&format!("\"pid\":{},", s.pid));
        out.push_str(&format!("\"port\":{},", s.port));
        out.push_str(&format!("\"host\":\"{}\",", json_escape(&s.host)));
        out.push_str(&format!("\"url\":\"{}\",", json_escape(&s.url)));
        out.push_str(&format!("\"name\":\"{}\",", json_escape(&s.name)));
        out.push_str(&format!("\"command\":\"{}\",", json_escape(&s.command)));
        out.push_str(&format!("\"cwd\":\"{}\",", json_escape(&s.cwd)));
        out.push_str(&format!("\"cwdShort\":\"{}\",", json_escape(&s.cwd_short)));
        out.push_str(&format!("\"project\":\"{}\",", json_escape(&s.project)));
        out.push_str(&format!("\"branch\":\"{}\",", json_escape(&s.branch)));
        out.push_str(&format!("\"isDev\":{}", if s.is_dev { "true" } else { "false" }));
        out.push('}');
    }
    out.push_str("]}");
    out
}

fn json_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                escaped.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => escaped.push(c),
        }
    }
    escaped
}

fn print_usage() {
    println!(
        "Usage:\n  omaport list [--all] [--json]\n  omaport kill <pid> [--port <port>] [--force]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_json_strings_safely() {
        let info = ServerInfo {
            pid: 4242,
            port: 3000,
            host: "127.0.0.1".into(),
            url: "http://localhost:3000".into(),
            name: "vite".into(),
            command: "node \"vite\" --host".into(),
            cwd: "/tmp/<img src=x>".into(),
            cwd_short: "/tmp/<img src=x>".into(),
            project: "<img src=x>".into(),
            branch: "feat/a\"b\\c".into(),
            is_dev: true,
        };
        let json = servers_to_json(&[info]);
        assert!(json.contains("\"pid\":4242"));
        assert!(json.contains("\"port\":3000"));
        assert!(json.contains("\"branch\":\"feat/a\\\"b\\\\c\""));
        assert!(json.contains("\"command\":\"node \\\"vite\\\" --host\""));
    }
}
