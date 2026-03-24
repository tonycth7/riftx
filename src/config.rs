// ~/.config/riftx/config.toml
//
// [core]
// parallel          = 8
// retry_count       = 3
// recursive         = false
// preserve_structure = false
// skip_existing     = true
// theme             = "amber"   # amber | dracula | nord | gruvbox | catppuccin | skyblue | tokyonight | ayu
//
// [auth]
// github_token   = "ghp_..."
// gitlab_token   = "glpat_..."
// codeberg_token = "..."
// gitea_token    = "..."
// gitea_url      = "https://git.example.com"
//
// [history]
// entries = [...]   # written automatically

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─── Theme name ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    #[default]
    Amber,
    Dracula,
    Nord,
    Gruvbox,
    Catppuccin,
    SkyBlue,
    TokyoNight,
    Ayu,
}

impl ThemeName {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "dracula"    => Self::Dracula,
            "nord"       => Self::Nord,
            "gruvbox"    => Self::Gruvbox,
            "catppuccin" => Self::Catppuccin,
            "skyblue"    => Self::SkyBlue,
            "tokyonight" => Self::TokyoNight,
            "ayu"        => Self::Ayu,
            _            => Self::Amber,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Amber      => "amber",
            Self::Dracula    => "dracula",
            Self::Nord       => "nord",
            Self::Gruvbox    => "gruvbox",
            Self::Catppuccin => "catppuccin",
            Self::SkyBlue    => "skyblue",
            Self::TokyoNight => "tokyonight",
            Self::Ayu        => "ayu",
        }
    }
    pub fn all() -> &'static [&'static str] {
        &["amber", "dracula", "nord", "gruvbox", "catppuccin", "skyblue", "tokyonight", "ayu"]
    }
}

// ─── Sub-tables ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoreConfig {
    #[serde(default = "default_parallel")]
    pub parallel: u8,

    /// How many times to retry a failed download before giving up
    #[serde(default = "default_retry")]
    pub retry_count: u8,

    /// Default: recursively expand directories in download plan
    #[serde(default)]
    pub recursive: bool,

    /// Default: preserve remote directory structure locally
    #[serde(default)]
    pub preserve_structure: bool,

    /// Default: skip files that already exist on disk
    #[serde(default = "default_true")]
    pub skip_existing: bool,

    #[serde(default)]
    pub theme: ThemeName,

    #[serde(default)]
    pub download_path: Option<PathBuf>,

    #[serde(default = "default_true")]
    pub cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    pub github_token:   Option<String>,
    pub gitlab_token:   Option<String>,
    pub codeberg_token: Option<String>,
    pub gitea_token:    Option<String>,
    pub gitea_url:      Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HistorySection {
    #[serde(default)]
    pub entries: Vec<HistoryEntry>,
}

fn default_parallel() -> u8  { 8 }
fn default_retry()    -> u8  { 3 }
fn default_true()     -> bool { true }

// ─── Top-level ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub history: HistorySection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub provider: String,
    pub owner:    String,
    pub repo:     String,
    pub branch:   String,
    #[serde(default)]
    pub instance: Option<String>,
}

impl Config {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("riftx")
            .join("config.toml")
    }

    pub fn load() -> Self {
        let p = Self::path();
        let mut cfg: Config = std::fs::read_to_string(&p)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default();

        if cfg.core.download_path.is_none() {
            cfg.core.download_path = dirs::download_dir()
                .or_else(dirs::home_dir)
                .or_else(|| Some(PathBuf::from(".")));
        }

        // Env-var overrides
        if cfg.auth.github_token.is_none()   { cfg.auth.github_token   = std::env::var("GITHUB_TOKEN").ok(); }
        if cfg.auth.gitlab_token.is_none()   { cfg.auth.gitlab_token   = std::env::var("GITLAB_TOKEN").ok(); }
        if cfg.auth.codeberg_token.is_none() { cfg.auth.codeberg_token = std::env::var("CODEBERG_TOKEN").ok(); }
        if cfg.auth.gitea_token.is_none()    { cfg.auth.gitea_token    = std::env::var("GITEA_TOKEN").ok(); }
        if cfg.auth.gitea_url.is_none()      { cfg.auth.gitea_url      = std::env::var("GITEA_URL").ok(); }

        cfg
    }

    pub fn save(&self) {
        let p = Self::path();
        if let Some(d) = p.parent() { let _ = std::fs::create_dir_all(d); }
        if let Ok(s) = toml::to_string_pretty(self) { let _ = std::fs::write(p, s); }
    }

    pub fn push_history(&mut self, entry: HistoryEntry) {
        self.history.entries.retain(|h| {
            !(h.provider == entry.provider && h.owner == entry.owner && h.repo == entry.repo)
        });
        self.history.entries.insert(0, entry);
        self.history.entries.truncate(20);
    }
}
