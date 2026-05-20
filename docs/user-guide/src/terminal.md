# Terminal

Hyprbole's default terminal is Ghostty. The shell is Bash with Hyprbole-owned startup files that load aliases, completions, prompt setup, runtime tooling, and a few quality-of-life defaults.

## Open Terminal

Open a terminal with:

```text
SUPER + RETURN
```

Open one from the app launcher with `SUPER + SPACE`, then search for Ghostty.

## Ghostty Defaults

Ghostty config lives at:

```text
~/.config/ghostty/config
```

Hyprbole's default Ghostty setup uses:

| Setting | Default |
| --- | --- |
| Font | `JetBrainsMono Nerd Font Mono` |
| Font size | `10` |
| Padding | `14px` horizontal and vertical |
| Cursor | Solid block, no blink |
| Theme | `~/.config/hyprbole/current/theme/ghostty.conf` |
| Scroll multiplier | `2.0` |

Useful keybinds:

| Key | Action |
| --- | --- |
| `Shift + Insert` | Paste from clipboard |
| `Control + Insert` | Copy to clipboard |
| `Control + Shift + V` | Paste from clipboard |
| `Control + Shift + C` | Copy to clipboard |

The Ghostty theme file under `current/theme` is generated runtime state. Edit source themes or `~/.config/ghostty/config` for local changes.

## Shell Startup

Hyprbole sources this Bash entrypoint:

```text
~/.local/share/hyprbole/default/bash/rc
```

That file loads:

| File | Purpose |
| --- | --- |
| `default/bash/envs` | Environment variables and PATH |
| `default/bash/shell` | Bash history, completion, and shell behavior |
| `default/bash/aliases` | Short aliases and helpers |
| `default/bash/functions` | Extra function loader |
| `default/bash/init` | Tool initialization |
| `default/bash/inputrc` | Readline completion and history search |

The installer adds a source line to `~/.bashrc`, so new interactive Bash shells load Hyprbole defaults automatically.

## Prompt: Starship

Hyprbole uses Starship for the shell prompt when the shell is interactive and the terminal is not `dumb`.

Starship config lives at:

```text
~/.config/starship.toml
```

Hyprbole's prompt is intentionally compact. It shows the current directory, Git branch, Git status, and a prompt character.

The shipped Starship format is:

```toml
format = "[$directory$git_branch$git_status]($style)$character"
```

Starship upstream documentation and source are here:

```text
https://github.com/starship/starship
```

After editing `~/.config/starship.toml`, open a new terminal or run:

```bash
source ~/.bashrc
```

## Common Aliases

Hyprbole adds these aliases and helpers:

| Alias | Action |
| --- | --- |
| `hb` | `hyprbole` |
| `g` | `git` |
| `c` | `opencode` |
| `..` | `cd ..` |
| `...` | `cd ../..` |
| `....` | `cd ../../..` |
| `ff` | `fzf` with `bat` preview |
| `n` | Open Neovim; without arguments, opens `nvim .` |

Hyprbole installs `eza`, so `ls`, `lsa`, `lt`, and `lta` use `eza` with icons and directory-first sorting.

## Navigation And Search

Hyprbole initializes `zoxide` when available. Use it to jump to directories you have visited before:

```bash
z project
zi
```

Hyprbole also loads `fzf` shell completion and key bindings when the package files exist. Use `ff` for an interactive file picker with a `bat` preview.

## Development Tooling

Hyprbole initializes `mise` when available:

```bash
mise activate bash
```

This lets projects manage language runtimes through `mise` without Hyprbole hardcoding per-language setup into the shell.

## Man Pages

Hyprbole sets man pages to render through `bat`:

```bash
man hyprctl
man bash
```

The relevant environment variables are:

```bash
MANROFFOPT="-c"
MANPAGER="sh -c 'col -bx | bat -l man -p'"
```

## Refreshing Config

Refresh the shipped Ghostty or Starship config from repo defaults with:

```bash
hb refresh-config ghostty/config
hb refresh-config starship.toml
```

Refreshing these files can overwrite local customizations after creating backups.
