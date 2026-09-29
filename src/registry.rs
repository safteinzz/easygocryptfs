//! `$XDG_CONFIG_HOME/egc/vaults` (usually `~/.config/egc/vaults`): every vault
//! egc has made or opened, one `<encrypted folder>\t<view>` line each, both
//! absolute.

use crate::vault::Vault;
use anyhow::{Result, bail};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

fn path() -> PathBuf {
    crate::config_dir().join("vaults")
}

/// The whole list. A missing file is an empty list; one that exists but cannot
/// be read is an error, because treating it as empty would let the next save
/// wipe it.
pub fn load() -> Result<Vec<Vault>> {
    load_from(&path())
}

fn load_from(file: &Path) -> Result<Vec<Vault>> {
    let text = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => bail!(
            "could not read `{}`: fix or remove it, egc will not overwrite it",
            crate::tilde(file)
        ),
    };
    Ok(text
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(cipher, view)| Vault {
            cipher: PathBuf::from(cipher),
            view: PathBuf::from(view),
        })
        .collect())
}

/// Add `v` unless it is already listed.
pub fn remember(v: &Vault) -> Result<()> {
    remember_in(&path(), v)
}

fn remember_in(file: &Path, v: &Vault) -> Result<()> {
    check_storable(v)?;
    let mut all = load_from(file)?;
    if all.iter().any(|known| known.cipher == v.cipher) {
        return Ok(());
    }
    all.push(v.clone());
    save_to(file, &all)
}

/// Fails for a vault whose path the one-line, tab-separated list cannot hold.
pub fn check_storable(v: &Vault) -> Result<()> {
    for p in [&v.cipher, &v.view] {
        if p.to_string_lossy().contains(['\t', '\n']) {
            bail!(
                "`{}` has a tab or a newline in its path, which egc's vault list cannot hold",
                crate::tilde(p)
            );
        }
    }
    Ok(())
}

/// Replace the whole list with `all`, through a temp file and a rename so an
/// interrupted save never leaves half a list.
pub fn save(all: &[Vault]) -> Result<()> {
    save_to(&path(), all)
}

fn save_to(file: &Path, all: &[Vault]) -> Result<()> {
    let body: String = all
        .iter()
        .map(|v| format!("{}\t{}\n", v.cipher.display(), v.view.display()))
        .collect();
    let dir = file.parent().unwrap_or(Path::new("."));
    if fs::create_dir_all(dir).is_err() {
        bail!(
            "could not create `{}` to save the vault list",
            crate::tilde(dir)
        );
    }
    let tmp = file.with_extension("tmp");
    if fs::write(&tmp, body).is_err() || fs::rename(&tmp, file).is_err() {
        let _ = fs::remove_file(&tmp);
        bail!(
            "could not write `{}` to save the vault list",
            crate::tilde(file)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn an_unreadable_vault_list_is_never_overwritten() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("egc-registry-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("vaults");
        // Not UTF-8, as a hand edit in the wrong encoding leaves it.
        let before = b"/example/.a\t/example/a\n\xff\n".to_vec();
        fs::write(&file, &before).unwrap();

        let v = Vault {
            cipher: PathBuf::from("/example/.b"),
            view: PathBuf::from("/example/b"),
        };
        let saved = remember_in(&file, &v);
        let after = fs::read(&file).unwrap();
        fs::remove_dir_all(&dir).unwrap();

        assert!(
            saved.is_err(),
            "adding to an unreadable list must fail, not start a new one"
        );
        assert_eq!(after, before, "the unreadable list was rewritten");
    }
}
