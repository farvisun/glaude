//! Agent processes Glaude started. Their owners stop them through async
//! tasks, which may never run again once the app is exiting, and a process
//! still in its startup handshake has no owner yet. App exit calls
//! [`kill_all`], which stops whatever is left synchronously.
//!
//! Children are spawned in their own process group (unix), so the whole
//! group is signalled: wrappers and their subprocesses go too.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

static PIDS: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());

/// Track a spawned agent process (its own process-group leader on unix).
pub fn register(pid: u32) {
    PIDS.lock().insert(pid);
}

/// Forget a process once it has exited and been reaped.
pub fn unregister(pid: u32) {
    PIDS.lock().remove(&pid);
}

/// Processes currently tracked.
pub fn tracked() -> Vec<u32> {
    PIDS.lock().iter().copied().collect()
}

/// Stop every tracked process: SIGTERM to the group, a short grace
/// period, then SIGKILL for anything left. Returns how many were signalled.
pub fn kill_all(grace: Duration) -> usize {
    let pids: Vec<u32> = std::mem::take(&mut *PIDS.lock()).into_iter().collect();
    if pids.is_empty() {
        return 0;
    }
    tracing::info!(count = pids.len(), "stopping agent processes");
    #[cfg(unix)]
    {
        let group = |pid: u32| -(pid as libc::pid_t);
        for &pid in &pids {
            unsafe { libc::kill(group(pid), libc::SIGTERM) };
        }
        let alive = |pid: u32| unsafe { libc::kill(group(pid), 0) } == 0;
        let deadline = Instant::now() + grace;
        while Instant::now() < deadline && pids.iter().any(|&p| alive(p)) {
            std::thread::sleep(Duration::from_millis(20));
        }
        for &pid in &pids {
            if alive(pid) {
                unsafe { libc::kill(group(pid), libc::SIGKILL) };
            }
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = (grace, Instant::now());
        for &pid in &pids {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .creation_flags(0x0800_0000)
                .status();
        }
    }
    pids.len()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::process::CommandExt;

    #[test]
    fn kills_tracked_groups_including_grandchildren() {
        // A shell that ignores SIGTERM and starts a grandchild: both must go.
        let mut child = std::process::Command::new("/bin/sh")
            .args(["-c", "trap '' TERM; sleep 30 & wait"])
            .process_group(0)
            .spawn()
            .unwrap();
        let pid = child.id();
        register(pid);
        std::thread::sleep(Duration::from_millis(150));
        let t = Instant::now();
        assert_eq!(kill_all(Duration::from_millis(200)), 1);
        assert!(t.elapsed() < Duration::from_secs(2));
        let status = child.wait().unwrap();
        assert!(!status.success());
        // Group is empty: the sleeping grandchild is gone too.
        std::thread::sleep(Duration::from_millis(50));
        assert_ne!(unsafe { libc::kill(-(pid as libc::pid_t), 0) }, 0);
        assert!(tracked().is_empty());
    }
}
