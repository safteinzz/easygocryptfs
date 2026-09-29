# easygocryptfs (`egc`)

> **Canonical:** [gitlab.com/safteinzz/easygocryptfs](https://gitlab.com/safteinzz/easygocryptfs) · **Mirror:** [github.com/safteinzz/easygocryptfs](https://github.com/safteinzz/easygocryptfs)

<!-- desc:start -->
gocryptfs without the flags - lock and unlock encrypted folders from wherever you stand
<!-- desc:end -->

## Install

```bash
cargo install easygocryptfs
egc self check   # is a newer release out?
egc self update  # install the latest
```

No cargo yet? Rust installs the same way on every distro: [rustup.rs](https://rustup.rs).

![A vault named photos made, unlocked, given a file, then locked, leaving photos empty and only encrypted names in .photos](https://gitlab.com/safteinzz/easygocryptfs/-/raw/main/readme-assets/egc.gif)

## It is still gocryptfs

Every vault is a plain gocryptfs folder, so gocryptfs opens it without egc:

```bash
gocryptfs .photos photos
```

egc only works out which folder you mean and types that line for you.

## Open the vault you are standing in

```bash
egc unlock           # asks for the password, opens it
egc                  # which vault is here, and is it open?
egc lock             # closes it again
```

`unlock` and `lock` walk up from the current folder to the nearest vault, so
they work from inside it, beside it, or anywhere below the folder that holds
it. A path picks one instead: `egc unlock ~/drive/photos`. Away from every
vault, `egc lock` closes the only one that is open.

## Lock it when you walk away

```bash
egc unlock -t 100    # locks itself after 100 minutes with nothing using it
```

The clock only runs while nothing is reading or writing, and a file still open
in the vault keeps it unlocked, so it never pulls a folder out from under you.

## Make a vault

```bash
egc init             # asks for a name, then makes .<name> (hidden) and <name>
egc init photos      # .photos, encrypted, and photos, where it opens
```

Enter at the name takes the default, `vault` unless you set another. gocryptfs
then asks for the new password twice and prints the vault's master key once:
write it down, it is the only way back in if you forget the password. A gocryptfs folder made before egc
works too once it is named `.<name>`, which is safe while it is locked because
gocryptfs never records its own folder's name.

## Keep track of every vault

```bash
egc ls               # every vault made or opened here, and its state
egc forget           # pick vaults by number to drop from that list
egc clear            # drop every missing one
```

A vault on an unplugged drive shows as `missing` rather than disappearing, and
`forget` and `clear` only ever drop lines from the list, never a vault's
folders.

## Commands

```bash
egc lock --all             # close every open vault
egc forget ~/drive/photos  # drop one by name, without the prompt
egc init --dir ~/drive     # make the vault somewhere else
```

Every command that changes something takes `-n` to print what it would do
instead; `egc <command> --help` has the rest.

## Where it keeps things

Both live in `~/.config/egc/`, or `$XDG_CONFIG_HOME/egc/` when that is set:

- `vaults`: the list `egc ls` shows, one vault per line.
- `settings`: `name = vault`, the name `egc init` offers.

## Compatibility

Linux with [gocryptfs](https://github.com/rfjakob/gocryptfs#installation) and fuse3. It reads `/proc`
to see what is open and closes vaults with `fusermount3`, so macOS and BSD need
a different implementation first.

## License

AGPL-3.0-only
