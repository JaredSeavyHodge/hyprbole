# Neovim

Hyprbole ships Neovim with LazyVim. LazyVim provides a modern editor setup on top of Neovim: file search, LSP integrations, completion, formatting hooks, Git helpers, and a discoverable keybinding layer.

## Open Neovim

Open a terminal with `SUPER + RETURN`, then run:

```bash
nvim
```

Open the current directory:

```bash
nvim .
```

Open a file:

```bash
nvim path/to/file
```

Hyprbole also provides the shell helper `n`, which opens `nvim .` when no file is passed:

```bash
n
n README.md
```

## First Launch

Hyprbole's Neovim config bootstraps `lazy.nvim` automatically from:

```text
https://github.com/folke/lazy.nvim.git
```

Then it loads LazyVim with:

```lua
{ "LazyVim/LazyVim", import = "lazyvim.plugins" }
```

The first launch may take a moment while plugins install.

## Beginner Movement

Neovim has modes. The two most important modes are Normal mode and Insert mode.

- Press `i` to enter Insert mode and type text.
- Press `Esc` to return to Normal mode.
- In Normal mode, use `h`, `j`, `k`, `l` to move left, down, up, and right.
- In Normal mode, type `:w` then Enter to save.
- In Normal mode, type `:q` then Enter to quit.
- Type `:wq` then Enter to save and quit.

Hyprbole also adds easier save and quit mappings:

| Key | Action |
| --- | --- |
| `Space w` | Write buffer |
| `Space q` | Quit window |
| `Space x` | Close buffer |
| `Esc` | Clear search highlight |

`Space` is the leader key.

## LazyVim Basics

LazyVim is designed around leader-key menus. Press `Space` and pause to see available mappings.

Useful starting points:

| Key | Action |
| --- | --- |
| `Space f f` | Find files |
| `Space f g` | Search text with grep |
| `Space e` | Toggle file explorer |
| `Space g g` | Open LazyGit when available |
| `Space l` | Language tooling menu |

These are LazyVim defaults. If a plugin has not finished installing yet, let Lazy finish and restart Neovim.

## Hyprbole Defaults

Hyprbole's editable Neovim files live under:

```text
~/.config/nvim
```

Important files:

| File | Purpose |
| --- | --- |
| `init.lua` | Loads the config |
| `lua/config/lazy.lua` | Bootstraps `lazy.nvim` and LazyVim |
| `lua/config/options.lua` | Editor options |
| `lua/config/keymaps.lua` | Hyprbole keymaps |
| `lua/config/autocmds.lua` | Autocommands |
| `lua/plugins/hyprbole-theme.lua` | Loads the current Hyprbole Neovim theme |

Hyprbole enables relative line numbers, mouse support, system clipboard integration, persistent undo, smart search case handling, cursor line, split preferences, and two-space indentation by default.

## Theme Integration

Hyprbole themes can ship Neovim theme data at:

```text
~/.config/hyprbole/current/theme/neovim.lua
```

That file is generated runtime state. Edit source theme files instead of editing `current/theme` directly.

## Plugin Management

LazyVim uses `lazy.nvim` for plugin management.

Open the plugin UI inside Neovim:

```vim
:Lazy
```

Common actions in the Lazy UI include update, sync, clean, and plugin status. Use these when LazyVim asks you to update plugins or when a plugin install fails.

## Diagnostics

Hyprbole adds a few diagnostic mappings:

| Key | Action |
| --- | --- |
| `Space e` | Show line diagnostics |
| `[d` | Previous diagnostic |
| `]d` | Next diagnostic |

LazyVim also provides more language tooling under the `Space l` menu.
