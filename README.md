# riftx v0.0.3

> ⚡ Blazing-fast CLI to explore, filter & extract files from remote repositories — **no clone needed.**
> Runs on any terminal, including headless servers over SSH.

```
 ██████╗ ██╗███████╗████████╗██╗  ██╗
 ██╔══██╗██║██╔════╝╚══██╔══╝╚██╗██╔╝
 ██████╔╝██║█████╗     ██║    ╚███╔╝
 ██╔══██╗██║██╔══╝     ██║    ██╔██╗
 ██║  ██║██║██║        ██║   ██╔╝╚██╗
 ╚═╝  ╚═╝╚═╝╚═╝        ╚═╝   ╚═╝  ╚═╝
```

> *«⚡ A fast interface between you and remote code/data»*

Built with **Rust · ratatui · tokio · reqwest**

---

## Install

```bash
# From source
git clone https://github.com/you/riftx
cd riftx
cargo build --release
# binary at ./target/release/riftx

# Or install via cargo
cargo install riftx
```

## Quick start

```bash
# Open the TUI home screen
riftx

# Jump straight into a repo
riftx https://github.com/rust-lang/rust

# Short form also works
riftx torvalds/linux

# Set your GitHub token (increases rate limit from 60 → 5000 req/hr)
riftx config set token ghp_xxxxxxxxxxxx

# Set a custom download directory
riftx config set path ~/Downloads/riftx

# View current config
riftx config list
```

---

## Features

| Feature | Details |
|---------|---------|
| **Full-screen TUI** | Navigate any public (or private with token) GitHub repo |
| **Inline file preview** | Read code side-by-side with the file list, with line numbers |
| **Branch switcher** | Press `b` to pop up a branch list and switch instantly |
| **Batch selection** | Space to toggle, `a` to select all, `u` to unselect |
| **Batch download** | `d` downloads all selected files at once, async |
| **Single download** | `D` downloads the highlighted file immediately |
| **Directory navigation** | Enter dirs, go back with `h`/Backspace, breadcrumb trail shown |
| **File filter / search** | Press `/` to fuzzy-filter files in the current directory |
| **Copy raw URL** | `c` copies the raw GitHub URL to clipboard |
| **Copy wget command** | `w` copies a ready-to-paste wget command |
| **Persistent history** | Last 10 repos remembered, press `↑` on home to fill input |
| **Config TUI** | Press `C` to open the settings screen in-app |
| **Headless-safe** | Runs perfectly over SSH on servers with no GUI |
| **Single binary** | No runtime deps — statically linkable |

---

## Keyboard reference

### Home screen

| Key | Action |
|-----|--------|
| Type | Build the repo URL |
| `Enter` | Load repo |
| `↑` | Fill input with most recent history |
| `C` | Open config |
| `q` / `Esc` | Quit |

### Browser

| Key | Action |
|-----|--------|
| `j` / `k` / `↑↓` | Navigate up / down |
| `Enter` / `l` / `→` | Enter directory or preview file |
| `h` / `Backspace` / `←` | Go back |
| `g` / `Home` | Jump to top |
| `G` / `End` | Jump to bottom |
| `Ctrl+d` / `Ctrl+u` | Page down / up |
| `Space` | Toggle select current item |
| `a` | Select all visible items |
| `u` | Unselect all |
| `d` | Download selected (or current file if none selected) |
| `D` | Download current file immediately |
| `p` | Toggle inline preview pane |
| `c` | Copy raw URL to clipboard |
| `w` | Copy wget command to clipboard |
| `/` | Filter files in current directory |
| `r` | Refresh current directory |
| `b` | Open branch switcher |
| `C` | Open config screen |
| `?` | Show help popup |
| `Esc` | Close preview → go back → home |
| `q` | Back to root / quit |
| `Ctrl+C` | Force quit |

### Preview pane

| Key | Action |
|-----|--------|
| `Ctrl+j` / `Ctrl+k` | Scroll preview down / up |
| `p` | Close preview |

---

## Configuration

Config is stored at `~/.config/riftx/config.json`.

```json
{
  "token": "ghp_...",
  "download_path": "/home/you/Downloads",
  "history": [...]
}
```

Set values from the CLI or via the in-app config screen (`C`):

```bash
riftx config set token ghp_xxxx   # GitHub personal access token
riftx config set path ~/Downloads  # Download destination
riftx config unset token           # Remove token
riftx config list                  # Show current config
```

---

## Roadmap

| Phase | Provider / Feature |
|-------|--------------------|
| ✅ v0.0.3 | GitHub — full TUI browser, preview, download, branch switch |
| 🔜 v0.1.x | GitLab, Bitbucket via Provider trait |
| 🔜 v0.2.x | Gitea (self-hosted), raw HTTP directories |
| 🔜 v0.3.x | ZIP/TAR URL extraction, caching layer, resume downloads |
| 🔜 future | Plugin system, S3, IPFS, public datasets |

---
## Acknowledgements

Inspired by [ghgrab](https://github.com/abhixdd/ghgrab), reimagined with a focus on performance, control, and extensibility.
## License

MIT
