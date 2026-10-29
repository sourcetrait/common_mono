//! The table from libproc: every pid from `proc_listallpids`, each one's BSD
//! record from `proc_pidinfo`, which answers for the caller's own user's live
//! processes alone.
use crate::Process;
use libc::{
    PROC_PIDTBSDINFO, SZOMB, c_char, c_int, proc_bsdinfo, proc_listallpids, proc_pidinfo,
};

pub(crate) fn processes() -> Vec<Process> {
    all_pids().into_iter().filter_map(process).collect()
}

pub(crate) fn process(pid: u32) -> Option<Process> {
    let info = bsd_info(pid)?;
    Some(Process {
        pid: info.pbi_pid,
        ppid: info.pbi_ppid,
        zombie: info.pbi_status == SZOMB,
        name: name_of(&info.pbi_comm),
        start_time: info
            .pbi_start_tvsec
            .checked_mul(1_000_000)?
            .checked_add(info.pbi_start_tvusec)?,
    })
}

/// Every pid, the buffer grown until the list fits with room to spare.
fn all_pids() -> Vec<u32> {
    // SAFETY: a null buffer of size zero asks for the count alone.
    let estimate = unsafe { proc_listallpids(std::ptr::null_mut(), 0) };
    let Ok(mut room) = usize::try_from(estimate) else {
        return Vec::new();
    };
    loop {
        room += room / 4 + 16;
        let mut pids: Vec<c_int> = vec![0; room];
        let Ok(size) = c_int::try_from(room * size_of::<c_int>()) else {
            return Vec::new();
        };
        // SAFETY: the buffer holds `size` bytes of c_int, and proc_listallpids
        // writes at most `size` bytes into it.
        let count = unsafe { proc_listallpids(pids.as_mut_ptr().cast(), size) };
        let Ok(count) = usize::try_from(count) else {
            return Vec::new();
        };
        if count < room {
            pids.truncate(count);
            return pids.into_iter().filter_map(|pid| u32::try_from(pid).ok()).collect();
        }
    }
}

/// One process's BSD record, none when its pid is not running.
fn bsd_info(pid: u32) -> Option<proc_bsdinfo> {
    let pid = c_int::try_from(pid).ok()?;
    let size = c_int::try_from(size_of::<proc_bsdinfo>()).ok()?;
    // SAFETY: proc_bsdinfo holds integers and byte arrays alone, so all zeros
    // is a valid value of it.
    let mut info: proc_bsdinfo = unsafe { std::mem::zeroed() };
    // SAFETY: the buffer is a proc_bsdinfo of exactly `size` bytes, and
    // proc_pidinfo writes at most `size` bytes into it.
    let written = unsafe { proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, (&raw mut info).cast(), size) };
    (written == size).then_some(info)
}

/// A NUL-terminated name, its bytes read as UTF-8 where they are.
fn name_of(comm: &[c_char]) -> String {
    let bytes: Vec<u8> = comm.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
    String::from_utf8_lossy(&bytes).into_owned()
}
