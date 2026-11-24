use std::collections::{HashMap, HashSet};
use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;

use crate::filter::{classify_dev_server, detect_label};
use crate::git::resolve_branch;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListeningSocket {
    pub inode: u64,
    pub host: String,
    pub port: u16,
    pub is_ipv6: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerInfo {
    pub pid: u32,
    pub port: u16,
    pub host: String,
    pub url: String,
    pub name: String,
    pub command: String,
    pub cwd: String,
    pub cwd_short: String,
    pub project: String,
    pub branch: String,
    pub is_dev: bool,
}

unsafe extern "C" {
    fn getuid() -> u32;
}

pub fn current_uid() -> u32 {
    unsafe { getuid() }
}

/// Scan `/proc/net/tcp` and `/proc/net/tcp6` for sockets owned by the current user
/// in `TCP_LISTEN` (`0A`) state, map them to PIDs via `/proc/<pid>/fd`, and enrich
/// each entry with cwd, command, framework name, and git branch.
pub fn scan_servers(include_all: bool) -> Vec<ServerInfo> {
    let uid = current_uid();
    let mut sockets = Vec::new();

    if let Ok(content) = fs::read_to_string("/proc/net/tcp") {
        sockets.extend(parse_proc_net_tcp(&content, false, Some(uid)));
    }
    if let Ok(content) = fs::read_to_string("/proc/net/tcp6") {
        sockets.extend(parse_proc_net_tcp(&content, true, Some(uid)));
    }

    if sockets.is_empty() {
        return Vec::new();
    }

    let inode_set: HashSet<u64> = sockets.iter().map(|s| s.inode).collect();
    let inode_to_pids = map_inodes_to_pids(&inode_set);

    let home = std::env::var("HOME").ok();

    // Deduplicate dual-stack bindings (same PID + same port on IPv4 & IPv6).
    let mut by_pid_port: HashMap<(u32, u16), ListeningSocket> = HashMap::new();
    for sock in sockets {
        let Some(pids) = inode_to_pids.get(&sock.inode) else {
            continue;
        };
        let Some(pid) = pick_primary_pid(pids) else {
            continue;
        };
        by_pid_port
            .entry((pid, sock.port))
            .and_modify(|existing| {
                // Prefer IPv4 host representation when both IPv4 and IPv6 are bound.
                if existing.is_ipv6 && !sock.is_ipv6 {
                    *existing = sock.clone();
                }
            })
            .or_insert(sock);
    }

    let mut branch_cache: HashMap<PathBuf, String> = HashMap::new();
    let mut servers = Vec::new();

    for ((pid, port), sock) in by_pid_port {
        let proc_dir = PathBuf::from(format!("/proc/{pid}"));
        let comm = fs::read_to_string(proc_dir.join("comm"))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let cmdline = read_cmdline(&proc_dir.join("cmdline"));
        let cwd_path = fs::read_link(proc_dir.join("cwd")).ok();

        let cwd = cwd_path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let cwd_short = shorten_home(&cwd, home.as_deref());
        let project = extract_project_name(&cwd, home.as_deref());

        let is_dev = classify_dev_server(&comm, &cmdline, port);
        if !include_all && !is_dev {
            continue;
        }

        let name = detect_label(&comm, &cmdline);
        let branch = match cwd_path {
            Some(ref path) => branch_cache
                .entry(path.clone())
                .or_insert_with(|| resolve_branch(path).unwrap_or_default())
                .clone(),
            None => String::new(),
        };

        let command = truncate_chars(
            if cmdline.is_empty() { &comm } else { &cmdline },
            160,
        );

        servers.push(ServerInfo {
            pid,
            port,
            host: sock.host,
            url: format!("http://localhost:{port}"),
            name,
            command,
            cwd,
            cwd_short,
            project,
            branch,
            is_dev,
        });
    }

    servers.sort_by(|a, b| a.port.cmp(&b.port).then_with(|| a.pid.cmp(&b.pid)));
    servers
}

/// Parse `/proc/net/tcp` or `/proc/net/tcp6` lines for listening sockets (`st == "0A"`).
pub fn parse_proc_net_tcp(
    content: &str,
    is_ipv6: bool,
    filter_uid: Option<u32>,
) -> Vec<ListeningSocket> {
    let mut out = Vec::new();
    for line in content.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 10 {
            continue;
        }
        // State 0A == TCP_LISTEN
        if cols[3] != "0A" {
            continue;
        }
        if let Some(want_uid) = filter_uid {
            let Ok(uid) = cols[7].parse::<u32>() else {
                continue;
            };
            if uid != want_uid {
                continue;
            }
        }
        let Ok(inode) = cols[9].parse::<u64>() else {
            continue;
        };
        if inode == 0 {
            continue;
        }
        let Some((host, port)) = parse_hex_addr(cols[1], is_ipv6) else {
            continue;
        };
        out.push(ListeningSocket {
            inode,
            host,
            port,
            is_ipv6,
        });
    }
    out
}

pub fn parse_hex_addr(token: &str, is_ipv6: bool) -> Option<(String, u16)> {
    let (addr_hex, port_hex) = token.split_once(':')?;
    let port = u16::from_str_radix(port_hex, 16).ok()?;

    if !is_ipv6 {
        if addr_hex.len() != 8 {
            return None;
        }
        let raw = u32::from_str_radix(addr_hex, 16).ok()?;
        let octets = raw.to_ne_bytes();
        let ip = Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]);
        Some((ip.to_string(), port))
    } else {
        if addr_hex.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 16];
        for i in 0..4 {
            let word = u32::from_str_radix(&addr_hex[i * 8..(i + 1) * 8], 16).ok()?;
            let word_bytes = word.to_ne_bytes();
            bytes[i * 4..(i + 1) * 4].copy_from_slice(&word_bytes);
        }
        let ip = Ipv6Addr::from(bytes);
        Some((ip.to_string(), port))
    }
}

fn map_inodes_to_pids(target_inodes: &HashSet<u64>) -> HashMap<u64, Vec<u32>> {
    let mut map: HashMap<u64, Vec<u32>> = HashMap::new();
    let Ok(entries) = fs::read_dir("/proc") else {
        return map;
    };

    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        let fd_dir = entry.path().join("fd");
        let Ok(fds) = fs::read_dir(fd_dir) else {
            continue;
        };
        for fd in fds.flatten() {
            let Ok(target) = fs::read_link(fd.path()) else {
                continue;
            };
            let Some(s) = target.to_str() else {
                continue;
            };
            if let Some(inner) = s.strip_prefix("socket:[").and_then(|v| v.strip_suffix(']')) {
                if let Ok(inode) = inner.parse::<u64>() {
                    if target_inodes.contains(&inode) {
                        map.entry(inode).or_default().push(pid);
                    }
                }
            }
        }
    }

    map
}

/// When a server forks workers that inherit the listening socket, prefer the
/// parent-most PID holding that socket so killing it stops the whole server.
fn pick_primary_pid(pids: &[u32]) -> Option<u32> {
    if pids.is_empty() {
        return None;
    }
    if pids.len() == 1 {
        return Some(pids[0]);
    }
    let set: HashSet<u32> = pids.iter().copied().collect();
    let mut roots: Vec<u32> = pids
        .iter()
        .copied()
        .filter(|&pid| read_ppid(pid).map(|ppid| !set.contains(&ppid)).unwrap_or(true))
        .collect();
    if roots.is_empty() {
        pids.iter().copied().min()
    } else {
        roots.sort_unstable();
        roots.first().copied()
    }
}

fn read_ppid(pid: u32) -> Option<u32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // comm is wrapped in parentheses and may itself contain spaces or ')'.
    let rparen = stat.rfind(')')?;
    let after = stat.get(rparen + 2..)?;
    let mut parts = after.split_whitespace();
    let _state = parts.next()?;
    let ppid = parts.next()?.parse::<u32>().ok()?;
    Some(ppid)
}

fn read_cmdline(path: &PathBuf) -> String {
    let Ok(bytes) = fs::read(path) else {
        return String::new();
    };
    if bytes.is_empty() {
        return String::new();
    }
    bytes
        .split(|&b| b == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn shorten_home(path: &str, home: Option<&str>) -> String {
    if let Some(h) = home {
        if !h.is_empty() {
            if path == h {
                return "~".to_string();
            }
            if let Some(rest) = path.strip_prefix(h) {
                if rest.starts_with('/') {
                    return format!("~{rest}");
                }
            }
        }
    }
    path.to_string()
}

fn extract_project_name(cwd: &str, home: Option<&str>) -> String {
    if cwd.is_empty() || cwd == "/" || home == Some(cwd) {
        return String::new();
    }
    PathBuf::from(cwd)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ipv4_and_ipv6_proc_net_tcp() {
        let tcp4 = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:1435 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 123456 1 0 100 0 0 10 0
   1: 00000000:0016 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 222222 1 0 100 0 0 10 0
   2: 0100007F:0BB8 0100007F:C350 01 00000000:00000000 00:00000000 00000000  1000        0 333333 1 0 100 0 0 10 0
";
        let res = parse_proc_net_tcp(tcp4, false, Some(1000));
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].host, "127.0.0.1");
        assert_eq!(res[0].port, 5173);
        assert_eq!(res[0].inode, 123456);

        let tcp6 = "\
  sl  local_address                         remote_address                        st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 00000000000000000000000001000000:0BB8 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 987654 1 0 100 0 0 10 0
";
        let res6 = parse_proc_net_tcp(tcp6, true, Some(1000));
        assert_eq!(res6.len(), 1);
        assert_eq!(res6[0].host, "::1");
        assert_eq!(res6[0].port, 3000);
        assert_eq!(res6[0].inode, 987654);
    }

    #[test]
    fn shortens_home_path() {
        assert_eq!(
            shorten_home("/home/xz/work/app", Some("/home/xz")),
            "~/work/app"
        );
        assert_eq!(shorten_home("/home/xz", Some("/home/xz")), "~");
        assert_eq!(shorten_home("/tmp/demo", Some("/home/xz")), "/tmp/demo");
    }
}
