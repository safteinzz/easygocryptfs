//! Whether a view is open, and closing it. Linux only: reads `/proc` and
//! shells out to `fusermount3`.

use anyhow::{Result, bail};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Whether `/proc/self/mounts` lists a gocryptfs mount on `view`. Anything
/// else mounted there is not this vault being open.
pub fn is_mounted(view: &Path) -> bool {
    let Ok(text) = fs::read_to_string("/proc/self/mounts") else {
        return false;
    };
    let want = view.to_string_lossy();
    text.lines().any(|l| {
        let mut fields = l.split(' ');
        let point = fields.nth(1);
        let kind = fields.next();
        kind == Some("fuse.gocryptfs") && point.is_some_and(|m| unescape(m) == want)
    })
}

/// A mount whose gocryptfs died stays listed but answers every read with
/// "Transport endpoint is not connected", so listed is not the same as open.
pub fn is_alive(view: &Path) -> bool {
    fs::read_dir(view).is_ok()
}

/// Unmount `view`, or with `lazy` detach it even while busy, which is only
/// for clearing a dead mount.
pub fn unmount(view: &Path, lazy: bool) -> Result<()> {
    let mut args = vec!["-u"];
    if lazy {
        args.push("-z");
    }
    let out = Command::new("fusermount3").args(&args).arg(view).output();
    let out = match out {
        Ok(out) => out,
        Err(_) => match Command::new("fusermount").args(&args).arg(view).output() {
            Ok(out) => out,
            Err(_) => {
                bail!("`fusermount3` is not installed: install `fuse3` with your package manager")
            }
        },
    };
    if out.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(&out.stderr);
    let said: String = said
        .lines()
        .filter(|l| !l.trim().is_empty())
        .take(5)
        .map(|l| format!("\n{l}"))
        .collect();
    if lazy {
        bail!(
            "could not clear the dead mount at `{}`{said}",
            crate::tilde(view)
        )
    }
    bail!(
        "could not lock `{}`: close whatever still has files open in it{said}",
        crate::tilde(view)
    )
}

/// The idle timeout the gocryptfs serving `view` was started with (`100m`),
/// read from its command line in `/proc`.
pub fn idle_timeout(view: &Path) -> Option<String> {
    let want = view.to_string_lossy();
    for entry in fs::read_dir("/proc").ok()?.flatten() {
        let Ok(raw) = fs::read(entry.path().join("cmdline")) else {
            continue;
        };
        let argv: Vec<String> = raw
            .split(|b| *b == 0)
            .map(|a| String::from_utf8_lossy(a).into_owned())
            .collect();
        let is_gocryptfs = argv
            .first()
            .is_some_and(|a| Path::new(a).file_name().is_some_and(|n| n == "gocryptfs"));
        if !is_gocryptfs || !argv.iter().any(|a| *a == want) {
            continue;
        }
        for (i, arg) in argv.iter().enumerate() {
            let flag = arg.trim_start_matches('-');
            if let Some((name, value)) = flag.split_once('=')
                && (name == "i" || name == "idle")
            {
                return Some(value.to_string());
            }
            if flag == "i" || flag == "idle" {
                return argv.get(i + 1).cloned();
            }
        }
        return None;
    }
    None
}

/// `/proc/mounts` writes a space, tab, newline or backslash in a path as a
/// three-digit octal escape (`\040`).
fn unescape(field: &str) -> String {
    let bytes = field.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let octal = bytes
            .get(i + 1..i + 4)
            .filter(|d| d.iter().all(|b| (b'0'..=b'7').contains(b)));
        if bytes[i] == b'\\'
            && let Some(digits) = octal
        {
            let n = digits
                .iter()
                .fold(0u8, |n, d| n.wrapping_mul(8) + (d - b'0'));
            out.push(n);
            i += 4;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
