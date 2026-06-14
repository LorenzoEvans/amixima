# Quickstart

Install from the repository:

```bash
cargo install --path .
```

Launch the TUI:

```bash
amixima tui /path/to/wavs
```

Or keep the compatibility shortcut:

```bash
amixima /path/to/wavs
```

Use `Tab` to move between panes, `Enter` to add effects, `Left` and `Right` to edit parameters, `p` to preview, `s` to save, and `Ctrl+Enter` to process the top-level WAV files in the current directory.

Use the CLI for non-interactive checks:

```bash
amixima validate examples/warm-bus.ini
amixima inspect examples/warm-bus.jsonld
amixima list-effects
```
