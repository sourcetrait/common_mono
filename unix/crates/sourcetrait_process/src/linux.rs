//! The table from `/proc`, one `stat` file a process.
use crate::Process;
use std::fs;

pub(crate) fn processes() -> Vec<Process> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.parse().ok())
        .filter_map(process)
        .collect()
}

pub(crate) fn process(pid: u32) -> Option<Process> {
    parse_stat(&fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)
}

/// One `stat` line. The name sits between the first `(` and the last `)`,
/// since it is unquoted and can hold spaces and parens of its own. The fields
/// after it count from the state, field 3, so the start time, field 22, is
/// the twentieth of them.
fn parse_stat(stat: &str) -> Option<Process> {
    let lparen = stat.find('(')?;
    let rparen = stat.rfind(')')?;
    let mut fields = stat.get(rparen + 1..)?.split_whitespace();
    let state = fields.next()?;
    let ppid = fields.next()?.parse().ok()?;
    Some(Process {
        pid: stat[..lparen].trim().parse().ok()?,
        ppid,
        zombie: state == "Z",
        name: stat.get(lparen + 1..rparen)?.to_string(),
        start_time: fields.nth(17)?.parse().ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_stat;

    const STAT: &str = "1234 (we (ird) name) S 1 1234 1234 0 -1 4194560 100 0 0 0 7 3 0 0 20 0 1 0 8675309 1000000 250";

    #[test]
    fn a_name_with_spaces_and_parens_is_read_whole() {
        let p = parse_stat(STAT).expect("parsed");
        assert_eq!((p.pid, p.ppid, p.name.as_str()), (1234, 1, "we (ird) name"));
        assert!(!p.zombie);
    }

    #[test]
    fn the_start_time_is_field_22() {
        assert_eq!(parse_stat(STAT).map(|p| p.start_time), Some(8675309));
    }

    #[test]
    fn state_z_is_a_zombie() {
        let zombie = STAT.replacen(") S ", ") Z ", 1);
        assert!(parse_stat(&zombie).expect("parsed").zombie);
    }

    #[test]
    fn a_truncated_or_malformed_line_is_none() {
        assert!(parse_stat("").is_none());
        assert!(parse_stat("no parens here").is_none());
        assert!(parse_stat("5 (x) Z 1 2 3").is_none());
        assert!(parse_stat("x (y) S 1 1 1 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 9 0 0").is_none());
        assert!(parse_stat("5 )x( S 1 1 1 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 9 0 0").is_none());
    }
}
