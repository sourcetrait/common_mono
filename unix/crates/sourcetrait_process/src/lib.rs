//! The host's process table: each process's parent, state, name, and start
//! time, read from `/proc` on Linux and from libproc on macOS.
//!
//! On macOS the table holds the caller's own user's live processes alone:
//! libproc refuses another user's record and has none for a zombie, so a
//! process leaves the table when it exits and `zombie` is never set there.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use self::linux as imp;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use self::macos as imp;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod other;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
use self::other as imp;

/// One process as the host's table shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    pub ppid: u32,
    /// Exited, its status not yet collected by its parent: Linux alone.
    pub zombie: bool,
    /// The executable's name as the kernel keeps it, cut to 15 bytes.
    pub name: String,
    /// When it started, in the platform's unit: clock ticks since boot on
    /// Linux, microseconds since the epoch on macOS. With the pid it names one
    /// process for as long as the host runs, since a reused pid starts later.
    pub start_time: u64,
}

/// Every process the table shows, in no particular order.
pub fn processes() -> Vec<Process> {
    imp::processes()
}

/// One process, none when its pid is not in the table.
pub fn process(pid: u32) -> Option<Process> {
    imp::process(pid)
}

/// A process's start time, none when its pid is not in the table.
pub fn start_time(pid: u32) -> Option<u64> {
    process(pid).map(|p| p.start_time)
}

/// Every descendant of `root` in the table, `root` itself excluded.
pub fn descendants(root: u32) -> Vec<u32> {
    descendants_in(&processes(), root)
}

/// Every descendant of `root` in `table`, `root` itself excluded.
pub fn descendants_in(table: &[Process], root: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(parent) = stack.pop() {
        for p in table {
            if p.ppid == parent && p.pid != root && !out.contains(&p.pid) {
                out.push(p.pid);
                stack.push(p.pid);
            }
        }
    }
    out
}
