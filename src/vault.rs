//! What a vault is on disk, and finding the one you mean from where you stand.
//!
//! A vault is two sibling folders: `.<name>`, the hidden encrypted one
//! gocryptfs owns (it holds `gocryptfs.conf`), and `<name>`, the folder it
//! opens into.

use anyhow::{Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Vault {
    pub cipher: PathBuf,
    pub view: PathBuf,
}

impl Vault {
    /// The vault opening into `view`.
    pub fn from_view(view: &Path) -> Vault {
        Vault {
            cipher: view.with_file_name(format!(".{}", file_name(view))),
            view: view.to_path_buf(),
        }
    }

    /// The vault whose encrypted folder is `cipher`, a dot folder.
    fn from_cipher(cipher: &Path) -> Vault {
        let name = file_name(cipher);
        Vault {
            cipher: cipher.to_path_buf(),
            view: cipher.with_file_name(name.trim_start_matches('.')),
        }
    }
}

/// The vault `start` is in, is, or sits beside: the nearest one walking up from
/// `start`. Fails when there is none, or when a folder on the way holds
/// several, rather than guessing; `verb` is the command the error suggests.
pub fn find(start: &Path, verb: &str) -> Result<Vault> {
    let Some(mut found) = nearest(start)? else {
        bail!(
            "no vault in or above `{}`: make one with `egc init`, or name its folder",
            crate::tilde(start)
        );
    };
    if found.len() == 1 {
        return Ok(found.remove(0));
    }
    let names: Vec<String> = found.iter().map(|v| file_name(&v.view)).collect();
    let dir = found[0].view.parent().unwrap_or(Path::new("/"));
    bail!(
        "`{}` holds several vaults ({}): name one, e.g. `egc {verb} {}`",
        crate::tilde(dir),
        names
            .iter()
            .map(|n| format!("`{n}`"))
            .collect::<Vec<_>>()
            .join(", "),
        crate::tilde(&found[0].view)
    )
}

/// Every vault at the nearest level walking up from `start` that has any:
/// one when `start` is inside or is a vault, all of them when a folder holds
/// several, `None` when no folder above has one.
pub fn nearest(start: &Path) -> Result<Option<Vec<Vault>>> {
    let Ok(start) = fs::canonicalize(start) else {
        // A view that was never made, or was removed while locked. Its parent
        // still exists, and resolving it keeps a symlinked path from becoming
        // a second name for the same vault.
        let abs = std::path::absolute(start).unwrap_or_else(|_| start.to_path_buf());
        let parent = abs.parent().and_then(|p| fs::canonicalize(p).ok());
        if let (Some(parent), Some(name)) = (parent, abs.file_name()) {
            let v = Vault::from_view(&parent.join(name));
            if is_cipher(&v.cipher) {
                return Ok(Some(vec![v]));
            }
        }
        bail!("`{}` does not exist", start.display());
    };
    for dir in start.ancestors() {
        if let Some(v) = at(dir) {
            return Ok(Some(vec![v]));
        }
        let found = ciphers_in(dir);
        if !found.is_empty() {
            return Ok(Some(found.iter().map(|c| Vault::from_cipher(c)).collect()));
        }
    }
    Ok(None)
}

/// `dir` itself as a vault, when it is a vault's encrypted folder or its view.
fn at(dir: &Path) -> Option<Vault> {
    if file_name(dir).starts_with('.') && is_cipher(dir) {
        return Some(Vault::from_cipher(dir));
    }
    dir.parent()?;
    let v = Vault::from_view(dir);
    is_cipher(&v.cipher).then_some(v)
}

/// Only dot folders are looked at, so a walk up through `/media` never stats a
/// mountpoint that might be a dead FUSE mount and hang.
fn ciphers_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| file_name(p).starts_with('.') && is_cipher(p))
        .collect();
    found.sort();
    found
}

fn is_cipher(dir: &Path) -> bool {
    dir.join("gocryptfs.conf").is_file()
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A throwaway folder under the temp dir, uniquely named per test and
    /// deleted by it, so no real vault is ever looked at.
    fn temp_dir(tag: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("egc-{tag}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::canonicalize(dir).unwrap()
    }

    /// A folder that passes for a vault's encrypted half: `find` only checks
    /// for a `gocryptfs.conf` in it.
    fn fake_vault(cipher: &Path) {
        fs::create_dir_all(cipher).unwrap();
        fs::write(cipher.join("gocryptfs.conf"), "{}").unwrap();
    }

    #[test]
    fn a_folder_holding_several_vaults_is_refused_not_guessed() {
        let dir = temp_dir("several");
        fake_vault(&dir.join(".a"));
        fake_vault(&dir.join(".b"));
        fs::create_dir(dir.join("sub")).unwrap();

        let found = nearest(&dir.join("sub")).unwrap().unwrap_or_default();
        let picked = find(&dir.join("sub"), "unlock");
        fs::remove_dir_all(&dir).unwrap();

        assert_eq!(
            found.len(),
            2,
            "both vaults beside the folder should be seen"
        );
        assert!(
            picked.is_err(),
            "with two vaults in reach, find must refuse rather than pick one"
        );
    }

    #[test]
    fn a_symlinked_path_resolves_to_the_vault_it_points_at() {
        let dir = temp_dir("symlink");
        fake_vault(&dir.join("other/.c"));
        std::os::unix::fs::symlink(dir.join("other"), dir.join("lnk")).unwrap();

        // `c` does not exist yet, the case where only its parent can be resolved.
        let v = find(&dir.join("lnk/c"), "unlock");
        fs::remove_dir_all(&dir).unwrap();

        let v = v.unwrap();
        assert_eq!(
            v.cipher,
            dir.join("other/.c"),
            "a link must not become a second name for the vault"
        );
        assert_eq!(v.view, dir.join("other/c"));
    }
}
