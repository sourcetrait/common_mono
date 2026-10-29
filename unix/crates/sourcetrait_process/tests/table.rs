//! The live table, read on every platform the crate serves.
use sourcetrait_process::{descendants, process, processes, start_time};
use std::{
    os::unix::process::parent_id,
    process::{Command, Stdio, id},
    thread,
    time::{Duration, Instant},
};

/// Poll `check` every 20 ms until it holds or five seconds pass.
fn within_five_seconds(mut check: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if check() {
            return true;
        }
        thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn our_own_process_is_in_the_table() {
    let me = process(id()).expect("our own process");
    assert_eq!((me.pid, me.ppid), (id(), parent_id()));
    assert!(!me.zombie);
    assert!(!me.name.is_empty());
    assert!(processes().iter().any(|p| p.pid == id()));
}

#[test]
fn a_start_time_is_stable_and_no_earlier_than_the_parents() {
    let ours = start_time(id()).expect("our start time");
    assert_eq!(start_time(id()), Some(ours));
    let parents = start_time(parent_id()).expect("our parent's start time");
    assert!(parents <= ours, "our parent started at {parents}, we at {ours}");
}

#[test]
fn a_pid_not_running_is_not_in_the_table() {
    assert!(process(u32::MAX - 1).is_none());
    assert!(start_time(u32::MAX - 1).is_none());
}

#[test]
fn a_child_and_its_own_child_are_descendants() {
    let mut child = Command::new("sh")
        .args(["-c", "sleep 30 & sleep 30"])
        .stdin(Stdio::null())
        .spawn()
        .expect("spawn sh");
    let parent = child.id();
    let grandchildren = || -> Vec<u32> {
        descendants(id())
            .into_iter()
            .filter(|&pid| process(pid).is_some_and(|p| p.ppid == parent))
            .collect()
    };
    let seen = within_five_seconds(|| descendants(id()).contains(&parent) && !grandchildren().is_empty());
    for pid in grandchildren() {
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
    }
    let _ = child.kill();
    let _ = child.wait();
    assert!(seen, "the child and its own child are descendants of the test");
}

#[cfg(target_os = "linux")]
#[test]
fn an_exited_child_not_yet_waited_is_a_zombie() {
    let mut child = Command::new("true").stdin(Stdio::null()).spawn().expect("spawn true");
    let pid = child.id();
    let mut started = None;
    let zombie = within_five_seconds(|| match process(pid) {
        Some(p) if p.zombie => {
            started = Some(p.start_time);
            true
        }
        _ => false,
    });
    let _ = child.wait();
    assert!(zombie, "the exited child shows as a zombie until it is waited");
    assert!(
        process(pid).is_none_or(|p| Some(p.start_time) != started),
        "the waited child leaves the table"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn an_exited_child_leaves_the_macos_table_at_its_exit() {
    let mut child = Command::new("sleep").arg("1").stdin(Stdio::null()).spawn().expect("spawn sleep");
    let pid = child.id();
    let alive = process(pid).is_some();
    let gone = within_five_seconds(|| process(pid).is_none());
    let _ = child.wait();
    assert!(alive && gone, "the child is in the table while it runs and leaves it at its exit, unwaited");
}

#[cfg(target_os = "macos")]
#[test]
fn another_users_process_is_not_in_the_macos_table() {
    let uid = Command::new("id").arg("-u").output().expect("run id");
    if String::from_utf8_lossy(&uid.stdout).trim() != "0" {
        assert!(process(1).is_none(), "launchd, root's, is not in a user's table");
    }
}

#[cfg(target_os = "macos")]
#[test]
fn a_macos_start_time_is_microseconds_since_the_epoch() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ours = start_time(id()).expect("our start time");
    let now = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH).expect("after the epoch").as_micros())
        .expect("fits");
    assert!(ours <= now && now - ours < 3_600_000_000, "started {ours}, now {now}");
}
