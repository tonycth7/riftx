use std::collections::HashSet;
use std::sync::Arc;
use ratatui::widgets::ListState;
use tokio::sync::{mpsc, Semaphore};

use crate::config::{Config, HistoryEntry, ThemeName};
use crate::fuzzy::{fuzzy_filter, FuzzyMatch};
use crate::providers::{
    self, ApiClient, Node, NodeKind, ProviderKind, RepoMeta,
    fmt_size, is_binary_ext, raw_url,
};
use crate::theme::Theme;

// ─── Screen ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Home,
    Browser,
    BranchPopup,
    DownloadPlan,
    Help,
    Config,
    Downloads,
}

// ─── Sort mode ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SortMode {
    Default,   // dirs first, then alpha
    Name,      // strict alpha
    Size,      // largest first
    Ext,       // group by extension
}

impl SortMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Name    => "name",
            Self::Size    => "size↓",
            Self::Ext     => "ext",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Default => Self::Name,
            Self::Name    => Self::Size,
            Self::Size    => Self::Ext,
            Self::Ext     => Self::Default,
        }
    }
}

// ─── Async messages ───────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum Msg {
    RepoLoaded(RepoMeta, Vec<String>),
    ContentsLoaded(String, Vec<Node>),
    PreviewLoaded(String),
    NodesDiscovered(Vec<Node>),
    DownloadDone { path: String, dest: String },
    DownloadFail { path: String, error: String },
    DownloadSkipped { path: String },
    ApiError(String),
}

// ─── Download entry ───────────────────────────────────────────────────────────

pub struct DlEntry {
    pub name:    String,
    pub path:    String,
    pub done:    bool,
    pub skipped: bool,
    pub error:   Option<String>,
}

// ─── Search mode ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SearchMode { Off, Name, Ext, Path }

// ─── Filtered item with fuzzy score ──────────────────────────────────────────

#[derive(Clone)]
pub struct FilteredItem {
    pub idx:   usize,
    pub fuzzy: Option<FuzzyMatch>,
}

// ─── Download plan ────────────────────────────────────────────────────────────

pub struct PlanItem {
    pub name: String,
    pub path: String,
    pub size: Option<u64>,
}

// ─── App ──────────────────────────────────────────────────────────────────────

pub struct App {
    pub screen: Screen,

    // Home input
    pub input:        String,
    pub input_cursor: usize,

    // Provider + repo
    pub provider:     ProviderKind,
    pub instance_url: Option<String>,
    pub owner:        String,
    pub repo:         String,
    pub branch:       String,
    pub repo_meta:    Option<RepoMeta>,
    pub branches:     Vec<String>,

    // File browser
    pub current_path: String,
    pub path_stack:   Vec<String>,
    pub files:        Vec<Node>,
    pub filtered:     Vec<FilteredItem>,
    pub list_state:   ListState,

    // Selection
    pub selected: HashSet<String>,

    // Bookmarks
    pub bookmarks: Vec<String>,  // paths pinned in current session

    // Search / filter
    pub search_mode:   SearchMode,
    pub search_query:  String,
    pub ext_filter:    Option<String>,
    pub min_size:      Option<u64>,  // byte threshold for size filter

    // Sort
    pub sort_mode: SortMode,

    // Preview
    pub preview:        Option<String>,
    pub preview_path:   Option<String>,
    pub preview_scroll: u16,

    // Branch popup
    pub branch_list_state: ListState,

    // Download plan + options
    pub plan:                 Vec<PlanItem>,
    pub dl_recursive:         bool,
    pub dl_preserve_structure: bool,
    pub dl_skip_existing:      bool,

    // Downloads
    pub downloads: Vec<DlEntry>,

    // Config screen
    pub cfg_field:   usize,
    pub cfg_editing: bool,
    pub cfg_buf:     String,

    // Autocomplete
    pub autocomplete_suggestions: Vec<String>,
    pub autocomplete_idx:         Option<usize>,

    // UI state
    pub status:  String,
    pub error:   Option<String>,
    pub loading: bool,

    // Tick counter for spinner animation
    pub tick: u64,

    // History
    pub history: Vec<HistoryEntry>,

    // Theme
    pub theme:      Theme,
    pub theme_name: ThemeName,

    // Persistent config + async
    pub config: Config,
    pub tx:     mpsc::Sender<Msg>,
    pub client: ApiClient,

    sem: Arc<Semaphore>,
}

impl App {
    pub fn new(tx: mpsc::Sender<Msg>, config: Config) -> Self {
        let theme_name = config.core.theme.clone();
        let theme      = Theme::get(&theme_name);
        let parallel   = config.core.parallel.max(1) as usize;
        let client = ApiClient::new(ProviderKind::GitHub, config.auth.github_token.clone(), None);
        let history = config.history.entries.clone();
        let dl_recursive          = config.core.recursive;
        let dl_preserve_structure = config.core.preserve_structure;
        let dl_skip_existing      = config.core.skip_existing;
        Self {
            screen: Screen::Home,
            input: String::new(), input_cursor: 0,
            provider: ProviderKind::GitHub, instance_url: None,
            owner: String::new(), repo: String::new(), branch: String::new(),
            repo_meta: None, branches: Vec::new(),
            current_path: String::new(), path_stack: Vec::new(),
            files: Vec::new(), filtered: Vec::new(),
            list_state: ListState::default(),
            selected: HashSet::new(),
            bookmarks: Vec::new(),
            search_mode: SearchMode::Off, search_query: String::new(),
            ext_filter: None, min_size: None,
            sort_mode: SortMode::Default,
            preview: None, preview_path: None, preview_scroll: 0,
            branch_list_state: ListState::default(),
            plan: Vec::new(),
            dl_recursive, dl_preserve_structure, dl_skip_existing,
            downloads: Vec::new(),
            cfg_field: 0, cfg_editing: false, cfg_buf: String::new(),
            autocomplete_suggestions: Vec::new(), autocomplete_idx: None,
            status: "  Enter a repo URL or owner/repo".into(),
            error: None, loading: false, tick: 0,
            history, theme, theme_name,
            config, tx, client,
            sem: Arc::new(Semaphore::new(parallel)),
        }
    }

    pub fn advance_tick(&mut self) { self.tick = self.tick.wrapping_add(1); }

    fn rebuild_client(&mut self) {
        let token = match self.provider {
            ProviderKind::GitHub   => self.config.auth.github_token.clone(),
            ProviderKind::GitLab   => self.config.auth.gitlab_token.clone(),
            ProviderKind::Codeberg => self.config.auth.codeberg_token.clone(),
            ProviderKind::Gitea    => self.config.auth.gitea_token.clone(),
        };
        self.client = ApiClient::new(self.provider, token, self.instance_url.as_deref());
        let parallel = self.config.core.parallel.max(1) as usize;
        self.sem = Arc::new(Semaphore::new(parallel));
    }

    // ── Fuzzy filter ──────────────────────────────────────────────────────────

    pub fn rebuild_filter(&mut self) {
        let ext = self.ext_filter.as_ref().map(|e| e.to_lowercase());
        let q   = self.search_query.to_lowercase();

        let names: Vec<String> = self.files.iter().map(|f| match self.search_mode {
            SearchMode::Path => f.path.clone(),
            SearchMode::Ext  => providers::file_ext(&f.name).to_string(),
            _                => f.name.clone(),
        }).collect();

        let scored = fuzzy_filter(&names, &q);

        let min_size = self.min_size;

        self.filtered = scored.into_iter()
            .filter(|(i, _)| {
                let f = &self.files[*i];
                if let Some(ref e) = ext {
                    if f.kind == NodeKind::File &&
                        providers::file_ext(&f.name).to_lowercase() != *e { return false; }
                }
                if let Some(min) = min_size {
                    if f.kind == NodeKind::File {
                        if f.size.unwrap_or(0) < min { return false; }
                    }
                }
                true
            })
            .map(|(idx, fm)| FilteredItem {
                idx,
                fuzzy: if q.is_empty() { None } else { Some(fm) },
            })
            .collect();

        // Apply sort within filtered
        self.apply_sort();

        let cur = self.list_state.selected().unwrap_or(0);
        self.list_state.select(
            if self.filtered.is_empty() { None }
            else { Some(cur.min(self.filtered.len() - 1)) }
        );
    }

    fn apply_sort(&mut self) {
        match self.sort_mode {
            SortMode::Default => {}  // providers already sort dirs-first alpha
            SortMode::Name => {
                self.filtered.sort_by(|a, b| {
                    self.files[a.idx].name.to_lowercase()
                        .cmp(&self.files[b.idx].name.to_lowercase())
                });
            }
            SortMode::Size => {
                self.filtered.sort_by(|a, b| {
                    let sa = self.files[a.idx].size.unwrap_or(0);
                    let sb = self.files[b.idx].size.unwrap_or(0);
                    sb.cmp(&sa)
                });
            }
            SortMode::Ext => {
                self.filtered.sort_by(|a, b| {
                    let ea = providers::file_ext(&self.files[a.idx].name).to_lowercase();
                    let eb = providers::file_ext(&self.files[b.idx].name).to_lowercase();
                    ea.cmp(&eb).then_with(|| {
                        self.files[a.idx].name.to_lowercase()
                            .cmp(&self.files[b.idx].name.to_lowercase())
                    })
                });
            }
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort_mode = self.sort_mode.next();
        self.rebuild_filter();
        self.status = format!("  sort → {}", self.sort_mode.label());
    }

    pub fn current_node(&self) -> Option<&Node> {
        self.list_state.selected()
            .and_then(|i| self.filtered.get(i))
            .and_then(|fi| self.files.get(fi.idx))
    }

    // ── Bookmarks ─────────────────────────────────────────────────────────────

    pub fn toggle_bookmark(&mut self) {
        if let Some(node) = self.current_node() {
            let p = node.path.clone();
            let name = node.name.clone();
            if let Some(pos) = self.bookmarks.iter().position(|b| b == &p) {
                self.bookmarks.remove(pos);
                self.status = format!("  ✗ unpinned: {name}");
            } else {
                self.bookmarks.push(p);
                self.status = format!("  ★ pinned: {name}");
            }
        }
    }

    pub fn is_bookmarked(&self, path: &str) -> bool {
        self.bookmarks.iter().any(|b| b == path)
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    pub fn nav_up(&mut self) {
        if self.filtered.is_empty() { return; }
        let i = self.list_state.selected().unwrap_or(0);
        self.list_state.select(Some(i.saturating_sub(1)));
    }
    pub fn nav_down(&mut self) {
        if self.filtered.is_empty() { return; }
        let i = self.list_state.selected().unwrap_or(0);
        if i + 1 < self.filtered.len() { self.list_state.select(Some(i + 1)); }
    }
    pub fn nav_top(&mut self)    { if !self.filtered.is_empty() { self.list_state.select(Some(0)); } }
    pub fn nav_bottom(&mut self) {
        if !self.filtered.is_empty() {
            self.list_state.select(Some(self.filtered.len() - 1));
        }
    }
    pub fn nav_page(&mut self, delta: i32) {
        if self.filtered.is_empty() { return; }
        let i   = self.list_state.selected().unwrap_or(0) as i32;
        let new = (i + delta).max(0).min(self.filtered.len() as i32 - 1) as usize;
        self.list_state.select(Some(new));
    }
    fn go_back(&mut self) {
        if let Some(prev) = self.path_stack.pop() { self.do_load_contents(prev); }
    }

    // ── Selection ─────────────────────────────────────────────────────────────

    pub fn toggle_select(&mut self) {
        if let Some(node) = self.current_node() {
            let p = node.path.clone();
            if !self.selected.remove(&p) { self.selected.insert(p); }
        }
    }
    pub fn select_all(&mut self) {
        for fi in &self.filtered {
            if let Some(f) = self.files.get(fi.idx) { self.selected.insert(f.path.clone()); }
        }
    }
    pub fn unselect_all(&mut self) { self.selected.clear(); }
    pub fn invert_selection(&mut self) {
        let visible: Vec<String> = self.filtered.iter()
            .filter_map(|fi| self.files.get(fi.idx))
            .map(|f| f.path.clone()).collect();
        for path in visible {
            if !self.selected.remove(&path) { self.selected.insert(path); }
        }
    }

    fn selected_files(&self) -> Vec<Node> {
        self.files.iter()
            .filter(|f| self.selected.contains(&f.path) && f.kind == NodeKind::File)
            .cloned().collect()
    }

    // ── Size filter ───────────────────────────────────────────────────────────

    pub fn cycle_size_filter(&mut self) {
        self.min_size = match self.min_size {
            None          => Some(1_024),           // > 1 KB
            Some(1_024)   => Some(100_000),         // > 100 KB
            Some(100_000) => Some(1_000_000),       // > 1 MB
            _             => None,
        };
        let label = match self.min_size {
            None                => "off".into(),
            Some(b)             => format!(">{}", fmt_size(b)),
        };
        self.status = format!("  size filter → {label}");
        self.rebuild_filter();
    }

    // ── Download plan ─────────────────────────────────────────────────────────

    pub fn open_download_plan(&mut self) {
        let files = self.selected_files();
        let dirs: Vec<Node> = if self.dl_recursive {
            self.files.iter()
                .filter(|f| self.selected.contains(&f.path) && f.kind == NodeKind::Dir)
                .cloned().collect()
        } else { Vec::new() };

        self.plan = if files.is_empty() && dirs.is_empty() {
            match self.current_node().cloned() {
                Some(n) if n.kind == NodeKind::File => vec![PlanItem {
                    name: n.name.clone(), path: n.path.clone(), size: n.size,
                }],
                Some(n) if n.kind == NodeKind::Dir && self.dl_recursive => {
                    vec![PlanItem { name: format!("{}/ (recursive)", n.name), path: n.path.clone(), size: None }]
                }
                _ => return,
            }
        } else {
            let mut items: Vec<PlanItem> = files.iter().map(|f| PlanItem {
                name: f.name.clone(), path: f.path.clone(), size: f.size,
            }).collect();
            for d in &dirs {
                items.push(PlanItem { name: format!("{}/ (recursive)", d.name), path: d.path.clone(), size: None });
            }
            items
        };
        self.screen = Screen::DownloadPlan;
    }

    pub fn plan_total_size(&self) -> u64 { self.plan.iter().filter_map(|p| p.size).sum() }

    pub fn execute_plan(&mut self) {
        let mut file_nodes: Vec<Node> = self.plan.iter()
            .filter(|pi| !pi.path.ends_with('/') && !pi.name.contains("(recursive)"))
            .filter_map(|pi| self.files.iter().find(|f| f.path == pi.path).cloned())
            .collect();

        let dir_nodes: Vec<Node> = self.plan.iter()
            .filter(|pi| pi.name.contains("(recursive)"))
            .filter_map(|pi| {
                let clean_path = pi.path.trim_end_matches('/').to_string();
                self.files.iter().find(|f| f.path == clean_path).cloned()
            })
            .collect();

        for dir in dir_nodes {
            let (tx, client) = (self.tx.clone(), self.client.clone());
            let (o, r, br)   = (self.owner.clone(), self.repo.clone(), self.branch.clone());
            tokio::spawn(async move {
                let nodes = collect_recursive(&client, &o, &r, &dir.path, &br).await;
                let _ = tx.send(Msg::NodesDiscovered(nodes)).await;
            });
        }

        for node in file_nodes.drain(..) {
            self.do_download_node(node);
        }

        self.screen = Screen::Downloads;
    }

    // ── Theme cycling ─────────────────────────────────────────────────────────

    pub fn next_theme(&mut self) {
        let all  = ThemeName::all();
        let cur  = self.theme_name.as_str();
        let pos  = all.iter().position(|&s| s == cur).unwrap_or(0);
        let next = all[(pos + 1) % all.len()];
        self.theme_name = ThemeName::from_str(next);
        self.theme      = Theme::get(&self.theme_name);
        self.config.core.theme = self.theme_name.clone();
        self.config.save();
        self.status = format!("  theme → {}", self.theme_name.as_str());
    }

    // ── Async actions ─────────────────────────────────────────────────────────

    pub fn do_load_repo(&mut self, kind: ProviderKind, owner: String, repo: String,
                        instance: Option<String>)
    {
        self.provider     = kind;
        self.instance_url = instance;
        self.owner        = owner.clone();
        self.repo         = repo.clone();
        self.loading      = true;
        self.error        = None;
        self.status       = format!("  [{kl}] Loading {owner}/{repo}…", kl = kind.label());
        self.rebuild_client();

        let (tx, client) = (self.tx.clone(), self.client.clone());
        let (o, r) = (owner.clone(), repo.clone());
        tokio::spawn(async move {
            match providers::get_repo(&client, &o, &r).await {
                Ok(meta) => {
                    let branches = providers::list_branches(&client, &o, &r).await.unwrap_or_default();
                    let _ = tx.send(Msg::RepoLoaded(meta, branches)).await;
                }
                Err(e) => { let _ = tx.send(Msg::ApiError(e.to_string())).await; }
            }
        });
    }

    pub fn do_load_contents(&mut self, path: String) {
        self.loading    = true;
        self.error      = None;
        self.ext_filter = None;
        let label = if path.is_empty() { "/".into() } else { path.clone() };
        self.status = format!("  Loading {label}…");

        let (tx, client) = (self.tx.clone(), self.client.clone());
        let (o, r, br, p) = (self.owner.clone(), self.repo.clone(), self.branch.clone(), path);
        tokio::spawn(async move {
            match providers::list_contents(&client, &o, &r, &p, &br).await {
                Ok(items) => { let _ = tx.send(Msg::ContentsLoaded(p, items)).await; }
                Err(e)    => { let _ = tx.send(Msg::ApiError(e.to_string())).await; }
            }
        });
    }

    pub fn do_load_preview(&mut self, node: Node) {
        if node.kind != NodeKind::File { return; }
        self.preview_path   = Some(node.path.clone());
        self.preview_scroll = 0;
        let size = node.size.unwrap_or(0);

        if is_binary_ext(&node.name) {
            self.preview = Some(format!(
                "  [binary: {}]\n  size: {}\n  path: {}\n\n  Press d to download.",
                node.name, fmt_size(size), node.path
            ));
            return;
        }
        if size > 512_000 {
            self.preview = Some(format!(
                "  [too large to preview]\n  size: {}\n\n  Press d to download.",
                fmt_size(size)
            ));
            return;
        }
        let Some(url) = node.download_url.clone() else {
            self.preview = Some("  [no download URL]".into());
            return;
        };
        self.preview = Some("  loading…".into());

        let (tx, client) = (self.tx.clone(), self.client.clone());
        tokio::spawn(async move {
            let result = client.get_text(&url).await;
            let _ = tx.send(Msg::PreviewLoaded(
                result.unwrap_or_else(|e| format!("  [error: {e}]"))
            )).await;
        });
    }

    pub fn do_download_node(&mut self, node: Node) {
        let Some(url) = node.download_url.clone() else { return };
        let dest_dir = self.config.core.download_path.clone()
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let dest = if self.dl_preserve_structure {
            let rel = std::path::PathBuf::from(&node.path);
            dest_dir.join(rel)
        } else {
            dest_dir.join(&node.name)
        };

        self.downloads.push(DlEntry {
            name: node.name.clone(), path: node.path.clone(),
            done: false, skipped: false, error: None,
        });
        self.status = format!("  ↓ {}", node.name);

        let (tx, client)   = (self.tx.clone(), self.client.clone());
        let sem            = self.sem.clone();
        let path           = node.path.clone();
        let skip_existing  = self.dl_skip_existing;
        let retry_count    = self.config.core.retry_count;

        tokio::spawn(async move {
            if skip_existing && dest.exists() {
                let _ = tx.send(Msg::DownloadSkipped { path }).await;
                return;
            }

            if let Some(parent) = dest.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }

            let _permit = sem.acquire().await;

            let mut last_err = String::new();
            for attempt in 0..=retry_count {
                if attempt > 0 {
                    let delay = std::time::Duration::from_millis(200 * (1u64 << attempt));
                    tokio::time::sleep(delay).await;
                }
                match client.get_bytes(&url).await {
                    Ok(bytes) => match tokio::fs::write(&dest, &bytes).await {
                        Ok(()) => {
                            let _ = tx.send(Msg::DownloadDone {
                                path, dest: dest.display().to_string(),
                            }).await;
                            return;
                        }
                        Err(e) => { last_err = e.to_string(); break; }
                    },
                    Err(e) => { last_err = e.to_string(); }
                }
            }
            let _ = tx.send(Msg::DownloadFail { path, error: last_err }).await;
        });
    }

    pub fn do_copy_url(&mut self) {
        let Some(node) = self.current_node() else { return };
        let url = raw_url(self.provider, self.instance_url.as_deref(),
                          &self.owner, &self.repo, &self.branch, &node.path);
        if try_clipboard(&url) { self.status = format!("  ✓ copied: {url}"); }
        else                   { self.status = format!("  url: {url}"); }
    }

    pub fn do_wget_cmd(&mut self) {
        let Some(node) = self.current_node() else { return };
        let url = raw_url(self.provider, self.instance_url.as_deref(),
                          &self.owner, &self.repo, &self.branch, &node.path);
        let cmd = format!("wget {url}");
        if try_clipboard(&cmd) { self.status = "  ✓ copied wget command".into(); }
        else                   { self.status = cmd; }
    }

    // ── Message handler ───────────────────────────────────────────────────────

    pub fn handle_msg(&mut self, msg: Msg) {
        self.loading = false;
        match msg {
            Msg::RepoLoaded(meta, branches) => {
                self.branch   = meta.def_branch.clone();
                self.branches = branches;
                let bi = self.branches.iter().position(|b| b == &self.branch).unwrap_or(0);
                self.branch_list_state.select(Some(bi));

                let full  = meta.full_name.clone();
                let stars = meta.stars;
                let lang  = meta.language.clone().unwrap_or_default();
                self.repo_meta = Some(meta);
                self.current_path.clear();
                self.path_stack.clear();
                self.files.clear(); self.filtered.clear();
                self.selected.clear(); self.preview = None;
                self.screen = Screen::Browser;
                self.ext_filter = None;
                self.min_size   = None;
                self.sort_mode  = SortMode::Default;
                self.bookmarks.clear();

                self.config.push_history(HistoryEntry {
                    provider: self.provider.label().to_lowercase(),
                    owner: self.owner.clone(), repo: self.repo.clone(),
                    branch: self.branch.clone(), instance: self.instance_url.clone(),
                });
                self.history = self.config.history.entries.clone();

                let lang_s = if lang.is_empty() { String::new() } else { format!("  {lang}") };
                self.status = format!("  {}  ⎇ {}  ★ {stars}{lang_s}", full, self.branch);
                self.do_load_contents(String::new());
            }

            Msg::ContentsLoaded(path, items) => {
                self.current_path = path;
                self.files        = items;
                self.search_query.clear();
                self.search_mode  = SearchMode::Off;
                self.list_state.select(if self.files.is_empty() { None } else { Some(0) });
                self.rebuild_filter();

                let dirs  = self.files.iter().filter(|f| f.kind == NodeKind::Dir).count();
                let files = self.files.len() - dirs;
                let disp  = if self.current_path.is_empty() { "/".into() }
                             else { format!("/{}", self.current_path) };
                self.status = format!("  {}/{}{disp}  {dirs}▸ {files}≡",
                    self.owner, self.repo);
            }

            Msg::PreviewLoaded(content) => { self.preview = Some(content); }

            Msg::NodesDiscovered(nodes) => {
                for node in nodes {
                    if node.kind == NodeKind::File {
                        let dest_dir = self.config.core.download_path.clone()
                            .unwrap_or_else(|| std::path::PathBuf::from("."));
                        let dest = if self.dl_preserve_structure {
                            dest_dir.join(std::path::PathBuf::from(&node.path))
                        } else {
                            dest_dir.join(&node.name)
                        };
                        let skip_existing = self.dl_skip_existing;
                        if !skip_existing || !dest.exists() {
                            self.do_download_node(node);
                        } else {
                            self.downloads.push(DlEntry {
                                name: node.name, path: node.path,
                                done: true, skipped: true, error: None,
                            });
                        }
                    }
                }
            }

            Msg::DownloadDone { path, dest } => {
                if let Some(d) = self.downloads.iter_mut().find(|d| d.path == path) {
                    d.done = true;
                }
                self.selected.remove(&path);
                let name = path.rsplit('/').next().unwrap_or(&path).to_string();
                self.status = format!("  ✓ {name}  →  {dest}");
            }

            Msg::DownloadFail { path, error } => {
                if let Some(d) = self.downloads.iter_mut().find(|d| d.path == path) {
                    d.error = Some(error.clone());
                }
                self.error = Some(format!("download failed: {error}"));
            }

            Msg::DownloadSkipped { path } => {
                if let Some(d) = self.downloads.iter_mut().find(|d| d.path == path) {
                    d.done    = true;
                    d.skipped = true;
                }
            }

            Msg::ApiError(e) => { self.error = Some(e); }
        }
    }

    // ── Autocomplete ──────────────────────────────────────────────────────────

    pub fn update_autocomplete(&mut self) {
        let q = self.input.trim().to_lowercase();
        if q.is_empty() {
            self.autocomplete_suggestions.clear();
            self.autocomplete_idx = None;
            return;
        }

        let mut suggestions: Vec<String> = Vec::new();

        for h in &self.history {
            let full_short = format!("{}/{}", h.owner, h.repo);
            let provider_prefix = match h.provider.as_str() {
                "gitlab"   => "gitlab.com",
                "codeberg" => "codeberg.org",
                "gitea"    => "gitea.com",
                _          => "github.com",
            };
            let full_url = format!("{}/{}/{}", provider_prefix, h.owner, h.repo);
            for candidate in [&full_short, &full_url] {
                if candidate.to_lowercase().starts_with(&q) || candidate.to_lowercase().contains(&q) {
                    if !suggestions.contains(candidate) { suggestions.push(candidate.clone()); }
                }
            }
            if suggestions.len() >= 6 { break; }
        }

        if suggestions.len() < 6 {
            for prefix in &["github.com/", "gitlab.com/", "codeberg.org/",
                             "https://github.com/", "https://gitlab.com/", "https://codeberg.org/"]
            {
                if prefix.starts_with(&q) && !suggestions.contains(&prefix.to_string()) {
                    suggestions.push(prefix.to_string());
                    if suggestions.len() >= 6 { break; }
                }
            }
        }

        self.autocomplete_suggestions = suggestions;
        if self.autocomplete_suggestions.is_empty() {
            self.autocomplete_idx = None;
        } else if let Some(idx) = self.autocomplete_idx {
            if idx >= self.autocomplete_suggestions.len() {
                self.autocomplete_idx = None;
            }
        }
    }

    // ── Key handler ───────────────────────────────────────────────────────────

    pub fn handle_key(
        &mut self,
        code: crossterm::event::KeyCode,
        mods: crossterm::event::KeyModifiers,
    ) -> bool {
        use crossterm::event::{KeyCode as KC, KeyModifiers as KM};

        if mods.contains(KM::CONTROL) {
            return match code {
                KC::Char('c') | KC::Char('q') => true,
                KC::Char('j') | KC::Down => {
                    self.preview_scroll = self.preview_scroll.saturating_add(5); false
                }
                KC::Char('k') | KC::Up => {
                    self.preview_scroll = self.preview_scroll.saturating_sub(5); false
                }
                KC::Char('d') => { self.nav_page(10);  false }
                KC::Char('u') => { self.nav_page(-10); false }
                KC::Char('t') => { self.next_theme();  false }
                _ => false,
            };
        }

        match self.screen.clone() {
            Screen::Home         => self.key_home(code),
            Screen::Browser      => self.key_browser(code),
            Screen::BranchPopup  => self.key_branch(code),
            Screen::DownloadPlan => self.key_plan(code),
            Screen::Help         => { self.screen = Screen::Browser; false }
            Screen::Config       => self.key_config(code),
            Screen::Downloads    => self.key_downloads(code),
        }
    }

    fn key_home(&mut self, code: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode as KC;
        match code {
            KC::Char(c) => {
                // Handle single-key shortcuts only when input is empty
                if self.input.is_empty() {
                    match c {
                        'q' | 'Q' => return true,
                        'C'       => { self.screen = Screen::Config; return false; }
                        'T'       => { self.next_theme(); return false; }
                        _ => {}
                    }
                    // Numeric shortcuts 1-6: load from history
                    if let '1'..='6' = c {
                        let idx = (c as usize) - ('1' as usize);
                        if let Some(h) = self.history.get(idx).cloned() {
                            let gitea_inst = self.config.auth.gitea_url.clone();
                            let url = format!(
                                "{}/{}/{}",
                                match h.provider.as_str() {
                                    "gitlab"   => "gitlab.com",
                                    "codeberg" => "codeberg.org",
                                    "gitea"    => "gitea.com",
                                    _          => "github.com",
                                },
                                h.owner, h.repo
                            );
                            if let Some((kind, owner, repo, inst)) =
                                providers::parse_url(&url, gitea_inst.as_deref())
                            {
                                self.do_load_repo(kind, owner, repo, inst);
                            }
                            return false;
                        }
                    }
                }
                self.input.insert(self.input_cursor, c);
                self.input_cursor += c.len_utf8();
                self.error = None;
                self.update_autocomplete();
                false
            }
            KC::Tab => {
                let idx = self.autocomplete_idx.unwrap_or(0);
                if let Some(s) = self.autocomplete_suggestions.get(idx).cloned() {
                    self.input        = s;
                    self.input_cursor = self.input.len();
                    self.autocomplete_suggestions.clear();
                    self.autocomplete_idx = None;
                }
                false
            }
            KC::Backspace => {
                if self.input_cursor > 0 {
                    let mut nc = self.input_cursor;
                    loop { nc -= 1; if self.input.is_char_boundary(nc) { break; } }
                    self.input.remove(nc);
                    self.input_cursor = nc;
                }
                self.update_autocomplete();
                false
            }
            KC::Left  => { if self.input_cursor > 0 { self.input_cursor -= 1; } false }
            KC::Right => {
                if self.input_cursor == self.input.len() && !self.autocomplete_suggestions.is_empty() {
                    let idx = self.autocomplete_idx.unwrap_or(0);
                    if let Some(s) = self.autocomplete_suggestions.get(idx).cloned() {
                        self.input        = s;
                        self.input_cursor = self.input.len();
                        self.autocomplete_suggestions.clear();
                        self.autocomplete_idx = None;
                    }
                } else if self.input_cursor < self.input.len() {
                    self.input_cursor += 1;
                }
                false
            }
            KC::Home => { self.input_cursor = 0; false }
            KC::End  => { self.input_cursor = self.input.len(); false }
            KC::Up => {
                if !self.autocomplete_suggestions.is_empty() {
                    self.autocomplete_idx = Some(match self.autocomplete_idx {
                        None | Some(0) => self.autocomplete_suggestions.len() - 1,
                        Some(i)        => i - 1,
                    });
                } else if let Some(h) = self.history.first() {
                    self.input        = format!("{}/{}", h.owner, h.repo);
                    self.input_cursor = self.input.len();
                    self.update_autocomplete();
                }
                false
            }
            KC::Down => {
                if !self.autocomplete_suggestions.is_empty() {
                    let n = self.autocomplete_suggestions.len();
                    self.autocomplete_idx = Some(match self.autocomplete_idx {
                        None    => 0,
                        Some(i) => (i + 1) % n,
                    });
                }
                false
            }
            KC::Enter => {
                if let Some(idx) = self.autocomplete_idx {
                    if let Some(s) = self.autocomplete_suggestions.get(idx).cloned() {
                        self.input        = s;
                        self.input_cursor = self.input.len();
                        self.autocomplete_suggestions.clear();
                        self.autocomplete_idx = None;
                        return false;
                    }
                }
                let s          = self.input.trim().to_string();
                let gitea_inst = self.config.auth.gitea_url.as_deref();
                if let Some((kind, owner, repo, inst)) = providers::parse_url(&s, gitea_inst) {
                    self.do_load_repo(kind, owner, repo, inst);
                } else if !s.is_empty() {
                    self.error = Some("invalid URL — try github.com/owner/repo".into());
                }
                false
            }
            KC::Esc => {
                if !self.autocomplete_suggestions.is_empty() {
                    self.autocomplete_suggestions.clear();
                    self.autocomplete_idx = None;
                    false
                } else if !self.input.is_empty() {
                    self.input.clear();
                    self.input_cursor = 0;
                    self.autocomplete_suggestions.clear();
                    self.autocomplete_idx = None;
                    false
                } else {
                    true
                }
            }
            KC::F(5) => { self.screen = Screen::Config; false }
            _ => false,
        }
    }

    fn key_browser(&mut self, code: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode as KC;

        if self.search_mode != SearchMode::Off {
            match code {
                KC::Char(c)   => { self.search_query.push(c);  self.rebuild_filter(); return false; }
                KC::Backspace => { self.search_query.pop();     self.rebuild_filter(); return false; }
                KC::Esc | KC::Enter => {
                    if code == KC::Esc { self.search_query.clear(); self.rebuild_filter(); }
                    self.search_mode = SearchMode::Off; return false;
                }
                _ => {}
            }
        }

        match code {
            KC::Char('q') | KC::Char('Q') => {
                if self.path_stack.is_empty() && self.current_path.is_empty() { return true; }
                self.path_stack.clear(); self.do_load_contents(String::new()); false
            }
            KC::Esc => {
                if self.preview.is_some() { self.preview = None; self.preview_path = None; }
                else if self.ext_filter.is_some() { self.ext_filter = None; self.rebuild_filter(); }
                else if !self.path_stack.is_empty() { self.go_back(); }
                else { self.screen = Screen::Home; }
                false
            }
            KC::Up    | KC::Char('k') => { self.nav_up();      false }
            KC::Down  | KC::Char('j') => { self.nav_down();    false }
            KC::Home  | KC::Char('g') => { self.nav_top();     false }
            KC::End   | KC::Char('G') => { self.nav_bottom();  false }
            KC::PageUp   => { self.nav_page(-10); false }
            KC::PageDown => { self.nav_page(10);  false }

            KC::Enter | KC::Char('l') | KC::Right => {
                if let Some(node) = self.current_node().cloned() {
                    if node.kind == NodeKind::Dir {
                        self.path_stack.push(self.current_path.clone());
                        self.do_load_contents(node.path.clone());
                    } else { self.do_load_preview(node); }
                }
                false
            }
            KC::Backspace | KC::Char('h') | KC::Left => {
                if self.preview.is_some() { self.preview = None; self.preview_path = None; }
                else { self.go_back(); }
                false
            }
            KC::Char(' ') => { self.toggle_select();    false }
            KC::Char('a') => { self.select_all();       false }
            KC::Char('u') => { self.unselect_all();     false }
            KC::Char('i') | KC::Char('I') => { self.invert_selection(); false }
            KC::Char('d') => { self.open_download_plan(); false }
            KC::Char('D') => {
                if let Some(node) = self.current_node().cloned() {
                    if node.kind == NodeKind::File { self.do_download_node(node); }
                }
                false
            }
            KC::Char('O') | KC::Char('o') => {
                if !self.downloads.is_empty() {
                    self.screen = Screen::Downloads;
                } else {
                    self.status = "  No downloads yet".into();
                }
                false
            }
            KC::Char('/') => { self.search_mode = SearchMode::Name; self.search_query.clear(); self.rebuild_filter(); false }
            KC::Char('%') => { self.search_mode = SearchMode::Ext;  self.search_query.clear(); self.rebuild_filter(); false }
            KC::Char('\\')=> { self.search_mode = SearchMode::Path; self.search_query.clear(); self.rebuild_filter(); false }
            KC::Char('x') | KC::Char('X') => {
                self.ext_filter = None; self.search_query.clear();
                self.search_mode = SearchMode::Off;
                self.min_size = None;
                self.rebuild_filter(); false
            }
            KC::Char('b') | KC::Char('B') => { self.screen = Screen::BranchPopup; false }
            KC::Char('r') | KC::Char('R') => { let p = self.current_path.clone(); self.do_load_contents(p); false }
            KC::Char('c') => { self.do_copy_url(); false }
            KC::Char('w') => { self.do_wget_cmd(); false }
            KC::Char('p') => {
                if self.preview.is_some() { self.preview = None; self.preview_path = None; }
                else if let Some(node) = self.current_node().cloned() {
                    if node.kind == NodeKind::File { self.do_load_preview(node); }
                }
                false
            }
            KC::Char('T') => { self.next_theme(); false }
            KC::Char('S') => { self.cycle_sort(); false }
            KC::Char('f') | KC::Char('F') => { self.cycle_size_filter(); false }
            KC::Char('m') => { self.toggle_bookmark(); false }
            KC::Char('n') => {
                // Return to home to load a new repo
                self.screen = Screen::Home;
                self.input.clear();
                self.input_cursor = 0;
                false
            }
            KC::Char('?') | KC::F(1) => { self.screen = Screen::Help;   false }
            KC::Char('C') => { self.screen = Screen::Config; false }
            KC::Char('e') => { self.error = None; false }
            _ => false,
        }
    }

    fn key_plan(&mut self, code: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode as KC;
        match code {
            KC::Enter | KC::Char('y') | KC::Char('Y') => { self.execute_plan(); false }
            KC::Char('r') | KC::Char('R') => {
                self.dl_recursive = !self.dl_recursive;
                let selected_backup = self.selected.clone();
                self.open_download_plan();
                self.selected = selected_backup;
                false
            }
            KC::Char('s') | KC::Char('S') => {
                self.dl_preserve_structure = !self.dl_preserve_structure; false
            }
            KC::Char('k') | KC::Char('K') => {
                self.dl_skip_existing = !self.dl_skip_existing; false
            }
            KC::Esc | KC::Char('q') | KC::Char('n') => { self.screen = Screen::Browser; false }
            _ => false,
        }
    }

    fn key_downloads(&mut self, code: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode as KC;
        match code {
            KC::Esc | KC::Char('q') | KC::Char('O') | KC::Char('o') => {
                self.screen = Screen::Browser; false
            }
            KC::Char('c') => {
                self.downloads.retain(|d| !d.done && d.error.is_none());
                false
            }
            _ => false,
        }
    }

    fn key_branch(&mut self, code: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode as KC;
        match code {
            KC::Up | KC::Char('k') => {
                let i = self.branch_list_state.selected().unwrap_or(0);
                self.branch_list_state.select(Some(i.saturating_sub(1))); false
            }
            KC::Down | KC::Char('j') => {
                let i = self.branch_list_state.selected().unwrap_or(0);
                if i + 1 < self.branches.len() { self.branch_list_state.select(Some(i + 1)); }
                false
            }
            KC::Enter => {
                if let Some(i) = self.branch_list_state.selected() {
                    if let Some(b) = self.branches.get(i).cloned() {
                        self.branch = b; self.path_stack.clear();
                        self.screen = Screen::Browser;
                        self.do_load_contents(String::new());
                    }
                }
                false
            }
            KC::Esc | KC::Char('q') | KC::Char('b') => { self.screen = Screen::Browser; false }
            _ => false,
        }
    }

    fn key_config(&mut self, code: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode as KC;
        if self.cfg_editing {
            match code {
                KC::Char(c)   => { self.cfg_buf.push(c); false }
                KC::Backspace => { self.cfg_buf.pop();   false }
                KC::Enter => {
                    self.apply_cfg_field();
                    self.config.save();
                    self.cfg_editing = false;
                    self.status = "  ✓ config saved".into();
                    false
                }
                KC::Esc => { self.cfg_editing = false; false }
                _ => false,
            }
        } else {
            match code {
                KC::Up | KC::Char('k')   => { if self.cfg_field > 0 { self.cfg_field -= 1; } false }
                KC::Down | KC::Char('j') => { self.cfg_field = (self.cfg_field + 1).min(5); false }
                KC::Enter => {
                    self.cfg_buf = self.cfg_field_value(self.cfg_field);
                    self.cfg_editing = true; false
                }
                KC::Esc | KC::Char('q') => {
                    self.screen = if self.repo_meta.is_some() { Screen::Browser } else { Screen::Home };
                    false
                }
                _ => false,
            }
        }
    }

    pub fn cfg_field_value_pub(&self, field: usize) -> String { self.cfg_field_value(field) }

    fn cfg_field_value(&self, field: usize) -> String {
        match field {
            0 => self.config.auth.github_token.clone().unwrap_or_default(),
            1 => self.config.auth.gitlab_token.clone().unwrap_or_default(),
            2 => self.config.auth.codeberg_token.clone().unwrap_or_default(),
            3 => self.config.auth.gitea_token.clone().unwrap_or_default(),
            4 => self.config.auth.gitea_url.clone().unwrap_or_default(),
            5 => self.config.core.download_path.as_ref()
                     .map(|p| p.display().to_string()).unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn apply_cfg_field(&mut self) {
        let val = if self.cfg_buf.is_empty() { None } else { Some(self.cfg_buf.clone()) };
        match self.cfg_field {
            0 => {
                self.config.auth.github_token = val.clone();
                if self.provider == ProviderKind::GitHub { self.rebuild_client(); }
            }
            1 => {
                self.config.auth.gitlab_token = val.clone();
                if self.provider == ProviderKind::GitLab { self.rebuild_client(); }
            }
            2 => {
                self.config.auth.codeberg_token = val.clone();
                if self.provider == ProviderKind::Codeberg { self.rebuild_client(); }
            }
            3 => {
                self.config.auth.gitea_token = val.clone();
                if self.provider == ProviderKind::Gitea { self.rebuild_client(); }
            }
            4 => { self.config.auth.gitea_url = val; }
            5 => {
                self.config.core.download_path =
                    val.map(|s| std::path::PathBuf::from(s))
                       .or_else(dirs::download_dir);
            }
            _ => {}
        }
    }
}

// ─── Recursive folder fetcher ─────────────────────────────────────────────────

async fn collect_recursive(
    client: &ApiClient,
    owner:  &str,
    repo:   &str,
    path:   &str,
    branch: &str,
) -> Vec<Node> {
    let mut result   = Vec::new();
    let mut queue    = vec![path.to_string()];
    let mut visited  = HashSet::new();
    const MAX_DIRS: usize = 200;

    while let Some(dir) = queue.pop() {
        if visited.len() >= MAX_DIRS { break; }
        if !visited.insert(dir.clone()) { continue; }

        let Ok(items) = providers::list_contents(client, owner, repo, &dir, branch).await
        else { continue; };

        for node in items {
            match node.kind {
                NodeKind::File => result.push(node),
                NodeKind::Dir  => queue.push(node.path),
            }
        }
    }
    result
}

// ─── Clipboard ────────────────────────────────────────────────────────────────

fn try_clipboard(text: &str) -> bool {
    use std::io::Write;
    for cmd in [
        &["xclip", "-selection", "clipboard"][..],
        &["xsel", "--clipboard", "--input"][..],
        &["wl-copy"][..],
        &["pbcopy"][..],
    ] {
        if let Ok(mut child) = std::process::Command::new(cmd[0])
            .args(&cmd[1..]).stdin(std::process::Stdio::piped()).spawn()
        {
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if child.wait().map(|s| s.success()).unwrap_or(false) { return true; }
        }
    }
    false
}
