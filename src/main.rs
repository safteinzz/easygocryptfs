//! easygocryptfs: gocryptfs without the flags. Binary: `egc`.
//!
//! This file is the clap `Cmd` enum, the dispatch match and the two path
//! helpers every module shares; what you can run is `egc --help`, rendered from
//! the manifest, these doc comments, `WAYS` and `AFTER`.
//!
//! Every command finds its vault from where you stand, and shells out to the
//! real `gocryptfs` and `fusermount3`, so vaults stay plain gocryptfs.

mod commands;
mod mounts;
mod registry;
mod selfcmd;
mod settings;
mod vault;

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

/// clap's own layout with `{before-help}` moved under `Usage:`, so the shapes
/// land on top of the command list rather than on top of the screen.
const TEMPLATE: &str =
    "{about-with-newline}\n{usage-heading} {usage}\n\n{before-help}{all-args}{after-help}\n";

const WAYS: &str = "\x1b[1mWays to run it (not subcommands):\x1b[0m
  egc    show the vault nearest this folder and its state";

const AFTER: &str = concat!(
    "\
A vault is two folders side by side: a hidden `.<name>`, which gocryptfs keeps
encrypted, and `<name>`, where it opens. `unlock` and `lock` walk up from the
current folder to the nearest one. `unlock` has no `--all`, since every vault
asks for its own password.

States, as `egc` and `egc ls` print them: open (unlocked), locked, dead
(gocryptfs died, `egc unlock` clears it), missing (its folder is gone, e.g.
an unplugged drive).

Output is for people, not data: results on stdout, errors on stderr as `egc: …`.
Exit 0 on success, 1 on failure, 2 on a usage error. gocryptfs reads passwords
from the terminal, or one line from stdin when it is piped.
Run `egc <command> --help` for a command's details.",
    "\n\n",
    env!("CARGO_PKG_REPOSITORY"),
    "\ncontributors: ",
    env!("CARGO_PKG_AUTHORS"),
);

/// `-V` stays a bare version string for scripts; `--version` adds the license,
/// the repository and who contributed, all from Cargo.toml.
const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "\n",
    env!("CARGO_PKG_LICENSE"),
    "  ",
    env!("CARGO_PKG_REPOSITORY"),
    "\ncontributors: ",
    env!("CARGO_PKG_AUTHORS"),
);

#[derive(Parser)]
#[command(
    name = "easygocryptfs",
    bin_name = "egc",
    version,
    long_version = LONG_VERSION,
    about,
    help_template = TEMPLATE,
    before_help = WAYS,
    after_help = AFTER
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Open the vault you are in or beside, asking its password  [PATH]
    ///   -t MINUTES   lock again after this long unused
    ///   -n           say what would happen, change nothing
    #[command(verbatim_doc_comment)]
    Unlock {
        /// A vault folder, or a folder above one (default: the current folder)
        path: Option<PathBuf>,
        /// Lock again after this many minutes with nothing using it
        #[arg(short, long, value_name = "MINUTES", value_parser = clap::value_parser!(u32).range(1..))]
        time: Option<u32>,
        /// Say what would happen, change nothing
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
    /// Close the vault you are in or beside, or the only open one  [PATH]
    ///   -a   close every open vault
    ///   -n   say what would happen, change nothing
    #[command(verbatim_doc_comment)]
    Lock {
        /// A vault folder, or a folder above one (default: the current folder)
        path: Option<PathBuf>,
        /// Close every open vault
        #[arg(short, long, conflicts_with = "path")]
        all: bool,
        /// Say what would happen, change nothing
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
    /// Make a new vault here, asking its name and password  [NAME]
    ///   --dir PATH   make it there instead (default: this folder)
    ///   -n           say what would happen, change nothing
    #[command(verbatim_doc_comment)]
    Init {
        /// What to call it (default: asked at a terminal, else the `name` setting, `vault` unless set)
        name: Option<String>,
        /// Where to make it (default: the current folder)
        #[arg(long, value_name = "PATH")]
        dir: Option<PathBuf>,
        /// Say what would happen, change nothing
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
    /// List every vault egc has made or opened, with its state
    Ls,
    /// Drop vaults from `egc ls`, asking which when none are named  [VAULT]...
    ///   -n   say what would happen, change nothing
    #[command(verbatim_doc_comment)]
    Forget {
        /// A vault as `egc ls` prints it; the folders themselves stay put
        vault: Vec<String>,
        /// Say what would happen, change nothing
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
    /// Drop every missing vault from `egc ls`
    ///   -n   say what would happen, change nothing
    #[command(verbatim_doc_comment)]
    Clear {
        /// Say what would happen, change nothing
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
    /// Manage easygocryptfs itself: `self update` reinstalls, `self check` looks for a newer release
    #[command(name = "self", subcommand)]
    Selfie(selfcmd::Cmd),
}

fn main() {
    let cli = Cli::parse();
    let done = match cli.command {
        None => commands::status(),
        Some(Cmd::Unlock {
            path,
            time,
            dry_run,
        }) => commands::unlock(path, time, dry_run),
        Some(Cmd::Lock { path, all, dry_run }) => commands::lock(path, all, dry_run),
        Some(Cmd::Init { name, dir, dry_run }) => commands::init(name, dir, dry_run),
        Some(Cmd::Ls) => commands::ls(),
        Some(Cmd::Forget { vault, dry_run }) => commands::forget(vault, dry_run),
        Some(Cmd::Clear { dry_run }) => commands::clear(dry_run),
        Some(Cmd::Selfie(cmd)) => selfcmd::run(cmd),
    };
    if let Err(e) = done {
        eprintln!("egc: {e}");
        std::process::exit(1);
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
        .join("egc")
}

/// `path` with the home folder written as `~`, for display only.
pub fn tilde(path: &Path) -> String {
    if let Some(home) = dirs::home_dir()
        && let Ok(rest) = path.strip_prefix(&home)
    {
        return Path::new("~").join(rest).display().to_string();
    }
    path.display().to_string()
}
