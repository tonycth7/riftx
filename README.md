<div align="center">

# ⎇ riftX

<p>
  <img src="https://readme-typing-svg.demolab.com?font=JetBrains+Mono&weight=500&size=24&duration=2400&pause=1000&color=CBA6F7&center=true&vCenter=true&width=700&lines=Blazing-fast+repo+explorer;No+clone+needed;Search+%2F+Download+%2F+Extract;Built+for+terminal+power+users" />
</p>
<p>
  <img src="https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust">
  <img src="https://img.shields.io/github/stars/tonycth7/riftx?style=flat">
  <img src="https://img.shields.io/github/license/tonycth7/riftx">
  <img src="https://img.shields.io/badge/TUI-Power-black">
</p>

```text
██████╗ ██╗███████╗████████╗██╗  ██╗
██╔══██╗██║██╔════╝╚══██╔══╝╚██╗██╔╝
██████╔╝██║█████╗     ██║    ╚███╔╝ 
██╔══██╗██║██╔══╝     ██║    ██╔██╗ 
██║  ██║██║██║        ██║   ██╔╝ ██╗
╚═╝  ╚═╝╚═╝╚═╝        ╚═╝   ╚═╝  ╚═╝
```
 </div>
⎇ explore repos like branches<br>
⇣ no clone, just pull what you need<br>
⌁ minimal, fast, terminal-native<br>
---

## Features

### 🔍 Browse
- Fuzzy file search (`/`), extension filter (`%`), full-path search (`\`)
- **Sort modes** — cycle with `S`: default (dirs-first) → name → size↓ → extension
- **Size filter** — cycle with `f`: off → >1 KB → >100 KB → >1 MB
- **Bookmarks / pins** — press `m` to pin important files (★ indicator in list)
- Inline file preview with line numbers (`p`)
- Branch switching (`b`), repo metadata (stars, language, private)
- Autocomplete from history on the home screen (`1`–`6` to instant-load)
- `n` key returns to home screen to load a new repo without quitting

### ⬇️ Smart Download Engine
- **Recursive folder download** — press `R` in the download plan
- **Preserve directory structure** — press `S` in plan to recreate remote paths locally
- **Skip existing files** — press `K` to avoid re-downloading
- **Parallel downloads** — configurable concurrency (default: 8)
- **Retry with exponential backoff** — up to `retry_count` times (default: 3)
- **Live progress panel** — press `O` to see all downloads with animated spinner
- **Instant single-file** — press `D` to download the highlighted file immediately

### 🎨 Themes
`amber` · `dracula` · `nord` · `gruvbox` · `catppuccin` · `skyblue` · `tokyonight` · `ayu`

Press `T` to cycle, or set in config / `--theme` flag.

---

## Install

```sh
cargo install --git https://github.com/tonycth7/riftx
```

Or build from source:

```sh
cargo build --release
./target/release/riftx
```

---

## Usage

```
riftx [URL]                   Launch TUI (home screen)
riftx browse <URL>            Browse a repo directly
riftx get    <URL>            Alias for browse
riftx config set   <key> <v> Save a config value
riftx config unset <key>     Remove a config value
riftx config list             Show current config
riftx --theme <n>             Override theme at launch
riftx --ext   <ext>           Pre-filter by extension
```

---

## Config

Located at `~/.config/riftx/config.toml`:

```toml
[core]
parallel           = 8      # max concurrent downloads
retry_count        = 3      # retries per failed file
recursive          = false  # default recursive mode
preserve_structure = false  # default structure mode
skip_existing      = true   # skip files already on disk
theme              = "amber"
download_path      = "/home/you/Downloads"

[auth]
github_token   = "ghp_..."
gitlab_token   = "glpat_..."
codeberg_token = "..."
gitea_token    = "..."
gitea_url      = "https://git.example.com"
```

Tokens are also read from env vars: `GITHUB_TOKEN`, `GITLAB_TOKEN`, `CODEBERG_TOKEN`, `GITEA_TOKEN`, `GITEA_URL`.

---

## Key Bindings

### Home Screen
| Key | Action |
|-----|--------|
| `Enter` | Load repo |
| `Tab / →` | Accept autocomplete |
| `↑ / ↓` | Navigate suggestions / history |
| `1`–`6` | Instantly load recent repo |
| `T` | Cycle theme |
| `C` | Config screen |
| `q / Esc` | Quit |

### Browser
| Key | Action |
|-----|--------|
| `j/k ↑↓` | Navigate |
| `Enter/l →` | Enter dir / preview file |
| `h/Bksp ←` | Go back |
| `g / G` | Top / bottom |
| `Ctrl+d / u` | Page down / up |
| `n` | New repo (go to home) |
| `Space` | Toggle select |
| `a / u / i` | Select all / none / invert |
| `/` | Fuzzy search by name |
| `%` | Filter by extension |
| `\` | Search by full path |
| `x` | Clear all filters |
| `S` | Cycle sort mode (default→name→size↓→ext) |
| `f` | Cycle size filter (off→1K→100K→1M) |
| `m` | Toggle bookmark / pin |
| `d` | Download plan popup |
| `D` | Instant download current file |
| `O` | Downloads progress panel |
| `p` | Toggle inline preview |
| `c / w` | Copy raw URL / wget command |
| `r` | Refresh directory |
| `b` | Switch branch |
| `T` | Cycle theme |
| `C` | Config screen |
| `?` | Help |
| `q / Esc` | Back / quit |

### In Download Plan
| Key | Action |
|-----|--------|
| `R` | Toggle recursive folder expansion |
| `S` | Toggle preserve directory structure |
| `K` | Toggle skip existing files |
| `Enter / y` | Execute plan |
| `Esc / n` | Cancel |

### In Downloads Panel
| Key | Action |
|-----|--------|
| `c` | Clear completed / failed entries |
| `Esc / O / q` | Close panel |

## Acknowledgements

Inspired by [ghgrab](https://github.com/abhixdd/ghgrab), reimagined with a focus on performance, control, and extensibility.
## License 
MIT
