use std::fs;
use std::thread;
use std::time::{Duration, Instant};

use crate::proc_scan::{current_uid, scan_servers};

const SIGTERM: i32 = 15;
const SIGKILL: i32 = 9;

unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

/// Safely terminate a listening server owned by the current user.
/// Verifies the PID is valid (> 1), owned by the current user, and (if `expected_port`
/// is supplied) is still listening on that port to guard against PID-reuse races.
pub fn kill_server(pid: u32, expected_port: Option<u16>, force_fallback: bool) -> Result<(), String> {
    if pid <= 1 {
        return Err(format!("refusing to signal protected PID {pid}"));
    }

    verify_owned_by_current_user(pid)?;

    if let Some(port) = expected_port {
        let servers = scan_servers(true);
        let still_matches = servers.iter().any(|s| s.pid == pid && s.port == port);
        if !still_matches {
            return Err(format!(
                "PID {pid} is no longer listening on port {port}"
            ));
        }
    }

    let pid_i32 = i32::try_from(pid).map_err(|_| format!("invalid PID {pid}"))?;
    let rc = unsafe { kill(pid_i32, SIGTERM) };
    if rc != 0 {
        let err = std::io::Error::last_os_error();
        return Err(format!("failed to send SIGTERM to PID {pid}: {err}"));
    }

    // Wait briefly for clean shutdown.
    let deadline = Instant::now() + Duration::from_millis(800);
    while Instant::now() < deadline {
        if !pid_exists(pid) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(40));
    }

    if force_fallback && pid_exists(pid) {
        let rc = unsafe { kill(pid_i32, SIGKILL) };
        if rc != 0 {
            let err = std::io::Error::last_os_error();
            // ESRCH means it exited right before SIGKILL.
            if err.raw_os_error() != Some(3) {
                return Err(format!("failed to send SIGKILL to PID {pid}: {err}"));
            }
        }
    }

    Ok(())
}

fn pid_exists(pid: u32) -> bool {
    fs::metadata(format!("/proc/{pid}")).is_ok()
}

fn verify_owned_by_current_user(pid: u32) -> Result<(), String> {
    let status = fs::read_to_string(format!("/proc/{pid}/status"))
        .map_err(|e| format!("cannot inspect PID {pid}: {e}"))?;
    let my_uid = current_uid();
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Uid:") {
            if let Some(real_uid) = rest.split_whitespace().next().and_then(|s| s.parse::<u32>().ok()) {
                if real_uid == my_uid {
                    return Ok(());
                }
                return Err(format!("PID {pid} is not owned by current user"));
            }
        }
    }
    Err(format!("could not determine owner of PID {pid}"))
}
