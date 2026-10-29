//! Elsewhere no table is read: no process is found, so none is proven live.
use crate::Process;

pub(crate) fn processes() -> Vec<Process> {
    Vec::new()
}

pub(crate) fn process(_pid: u32) -> Option<Process> {
    None
}
