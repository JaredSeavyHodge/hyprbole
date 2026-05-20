# Tools

Hyprbole's Tools menu groups useful local utilities that are not primary apps. The first tool is Disk Usage, powered by `dua-cli`.

## Open Tools

Open the Hyprbole menu:

```text
SUPER + ALT + SPACE
```

Then choose `Tools`.

From a terminal, use:

```bash
hb tools
```

## Disk Usage

`Disk Usage` opens `dua i` in the default terminal. `dua` is a fast terminal disk-usage browser.

Open it from the Tools menu, search for `Disk Usage` in the app launcher, or run:

```bash
hb disk-usage
```

From a terminal, `hb disk-usage` runs `dua i` in the current terminal. Launcher and menu entries use `hb launch disk-usage` so a terminal window opens for the tool.

To inspect a specific path, pass it to Hyprbole:

```bash
hb disk-usage ~/Downloads
hb disk-usage ~/.local/share/hyprbole
```

You can also run `dua` directly:

```bash
dua i
dua i ~/Downloads
```

## Launcher Entry

Hyprbole generates this desktop entry so Walker's app launcher can find Disk Usage:

```text
~/.local/share/applications/hyprbole-disk-usage.desktop
```

Refresh generated tool launchers with:

```bash
hyprbole-refresh-tool-launchers
```
