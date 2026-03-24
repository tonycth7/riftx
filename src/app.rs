use std::collections::HashSet;
use ratatui::widgets::ListState;
use tokio::sync::mpsc;

use crate::config::{Config, HistoryEntry};
use crate::github::{fmt_size, is_binary_ext, raw_url, GhClient, GhItem, RepoInfo};

// ─── Screen ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Home,
    Browser,
    BranchPopup,
    Help,
    Config,
}

// ─── Background messages ──────────────────────────────────────────────────────

#[derive(Debug)]
pub enum Msg {
    RepoLoaded(RepoInfo, Vec<String>),
    ContentsLoaded(String, Vec<GhItem>),
    PreviewLoaded(String),
    DownloadDone { path: String, dest: String },
    DownloadFail { path: String, error: String },
    ApiError(String),
}

// ─── Download tracking ────────────────────────────────────────────────────────

pub struct DlEntry {
    pub name:  String,
    pub path:  String,
    pub done:  bool,
    pub error: Option<String>,
}

// ─── App ─────────────────────────────────────────────────────────────────────

pub struct App {
    // ── Screen
    pub screen: Screen,

    // ── Home input
    pub input:        String,
    pub input_cursor: usize,

    // ── Repo data
    pub owner:     String,
    pub repo:      String,
    pub branch:    String,
    pub repo_info: Option<RepoInfo>,
    pub branches:  Vec<String>,

    // ── File browser
    pub current_path: String,
    pub path_stack:   Vec<String>,
    pub files:        Vec<GhItem>,
    pub filtered:     Vec<usize>,    // indices into files
    pub list_state:   ListState,

    // ── Selection
    pub selected: HashSet<String>,   // file paths

    // ── Search / filter
    pub search_active: bool,
    pub search_query:  String,

    // ── Preview pane
    pub preview:      Option<String>,  // file content
    pub preview_path: Option<String>,  // path being previewed
    pub preview_scroll: u16,

    // ── Branch popup
    pub branch_list_state: ListState,

    // ── Downloads
    pub downloads: Vec<DlEntry>,

    // ── Config screen state
    pub cfg_field:   usize,   // 0 = token, 1 = path
    pub cfg_editing: bool,
    pub cfg_buf:     String,

    // ── UI state
    pub status:  String,
    pub error:   Option<String>,
    pub loading: bool,

    // ── History shown on home
    pub history: Vec<HistoryEntry>,

    // ── Persistent config
    pub config: Config,

    // ── Async
    pub tx: mpsc::Sender<Msg>,
    pub gh: GhClient,
}

impl App {
    pub fn new(tx: mpsc::Sender<Msg>, config: Config) -> Self {
        let gh = GhClient::new(config.token.clone());
        let history = config.history.clone();
        Self {
            screen: Screen::Home,
            input: String::new(),
            input_cursor: 0,
            owner: String::new(),
            repo: String::new(),
            branch: String::new(),
            repo_info: None,
            branches: Vec::new(),
            current_path: String::new(),
            path_stack: Vec::new(),
            files: Vec::new(),
            filtered: Vec::new(),
            list_state: ListState::default(),
            selected: HashSet::new(),
            search_active: false,
            search_query: String::new(),
            preview: None,
            preview_path: None,
            preview_scroll: 0,
            branch_list_state: ListState::default(),
            downloads: Vec::new(),
            cfg_field: 0,
            cfg_editing: false,
            cfg_buf: String::new(),
            status: String::from("  Enter a GitHub URL or owner/repo"),
            error: None,
            loading: false,
            history,
            config,
            tx,
            gh,
        }
    }

    // ── Filtering ─────────────────────────────────────────────────────────────

    pub fn rebuild_filter(&mut self) {
        let q = self.search_query.to_lowercase();
        self.filtered = if q.is_empty() {
            (0..self.files.len()).collect()
        } else {
            self.files.iter().enumerate()
                .filter(|(_, f)| f.name.to_lowercase().contains(&q))
                .map(|(i, _)| i)
                .collect()
        };
        let cur = self.list_state.selected().unwrap_or(0);
        self.list_state.select(if self.filtered.is_empty() { None }
            else { Some(cur.min(self.filtered.len() - 1)) });
    }

    pub fn current_item(&self) -> Option<&GhItem> {
        self.list_state.selected()
            .and_then(|i| self.filtered.get(i))
            .and_then(|&fi| self.files.get(fi))
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
        if i + 1 < self.filtered.len() {
            self.list_state.select(Some(i + 1));
        }
    }

    pub fn nav_top(&mut self) {
        if !self.filtered.is_empty() { self.list_state.select(Some(0)); }
    }

    pub fn nav_bottom(&mut self) {
        if !self.filtered.is_empty() {
            self.list_state.select(Some(self.filtered.len() - 1));
        }
    }

    pub fn nav_page(&mut self, delta: i32) {
        if self.filtered.is_empty() { return; }
        let i = self.list_state.selected().unwrap_or(0) as i32;
        let new = (i + delta).max(0).min(self.filtered.len() as i32 - 1) as usize;
        self.list_state.select(Some(new));
    }

    fn go_back(&mut self) {
        if let Some(prev) = self.path_stack.pop() {
            self.do_load_contents(prev);
        }
    }

    // ── Selection ─────────────────────────────────────────────────────────────

    pub fn toggle_select(&mut self) {
        if let Some(item) = self.current_item() {
            let p = item.path.clone();
            if !self.selected.remove(&p) { self.selected.insert(p); }
        }
    }

    pub fn select_all(&mut self) {
        for &fi in &self.filtered {
            if let Some(f) = self.files.get(fi) {
                self.selected.insert(f.path.clone());
            }
        }
    }

    pub fn unselect_all(&mut self) { self.selected.clear(); }

    fn selected_files(&self) -> Vec<GhItem> {
        self.files.iter()
            .filter(|f| self.selected.contains(&f.path) && f.kind == "file")
            .cloned()
            .collect()
    }

    // ── Async actions ─────────────────────────────────────────────────────────

    pub fn do_load_repo(&mut self, owner: String, repo: String) {
        self.loading = true;
        self.error = None;
        self.status = format!("  Loading {owner}/{repo}…");
        self.owner = owner.clone();
        self.repo  = repo.clone();

        let (tx, gh) = (self.tx.clone(), self.gh.clone());
        tokio::spawn(async move {
            match gh.get_repo(&owner, &repo).await {
                Ok(info) => {
                    let branches = gh.get_branches(&owner, &repo).await.unwrap_or_default();
                    let _ = tx.send(Msg::RepoLoaded(info, branches)).await;
                }
                Err(e) => { let _ = tx.send(Msg::ApiError(e.to_string())).await; }
            }
        });
    }

    pub fn do_load_contents(&mut self, path: String) {
        self.loading = true;
        self.error = None;
        let label = if path.is_empty() { "/".into() } else { path.clone() };
        self.status = format!("  Loading {label}…");

        let (tx, gh) = (self.tx.clone(), self.gh.clone());
        let (owner, repo, branch, p) =
            (self.owner.clone(), self.repo.clone(), self.branch.clone(), path);
        tokio::spawn(async move {
            match gh.get_contents(&owner, &repo, &p, &branch).await {
                Ok(items) => { let _ = tx.send(Msg::ContentsLoaded(p, items)).await; }
                Err(e)    => { let _ = tx.send(Msg::ApiError(e.to_string())).await; }
            }
        });
    }

    pub fn do_load_preview(&mut self, item: GhItem) {
        if item.kind != "file" { return; }

        self.preview_path   = Some(item.path.clone());
        self.preview_scroll = 0;

        let size = item.size.unwrap_or(0);

        if is_binary_ext(&item.name) {
            self.preview = Some(format!(
                "  [binary file: {}]\n\n  size:  {}\n  path:  {}\n\n  Press d to download.",
                item.name, fmt_size(size), item.path
            ));
            return;
        }
        if size > 512_000 {
            self.preview = Some(format!(
                "  [file too large to preview]\n\n  size:  {}\n  path:  {}\n\n  Press d to download.",
                fmt_size(size), item.path
            ));
            return;
        }

        let Some(url) = item.download_url.clone() else {
            self.preview = Some("  [no download URL]".to_string());
            return;
        };

        self.preview = Some("  loading preview…".to_string());

        let (tx, gh) = (self.tx.clone(), self.gh.clone());
        tokio::spawn(async move {
            match gh.get_text(&url).await {
                Ok(t)  => { let _ = tx.send(Msg::PreviewLoaded(t)).await; }
                Err(e) => { let _ = tx.send(Msg::PreviewLoaded(format!("  [error: {e}]"))).await; }
            }
        });
    }

    pub fn do_download_item(&mut self, item: GhItem) {
        let Some(url) = item.download_url.clone() else { return };
        let dest_dir = self.config.download_path.clone()
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let dest = dest_dir.join(&item.name);

        self.downloads.push(DlEntry {
            name: item.name.clone(),
            path: item.path.clone(),
            done: false, error: None,
        });
        self.status = format!("  Downloading {}…", item.name);

        let (tx, gh) = (self.tx.clone(), self.gh.clone());
        let path = item.path.clone();
        tokio::spawn(async move {
            match gh.get_bytes(&url).await {
                Ok(bytes) => match std::fs::write(&dest, &bytes) {
                    Ok(()) => {
                        let _ = tx.send(Msg::DownloadDone {
                            path, dest: dest.display().to_string()
                        }).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Msg::DownloadFail { path, error: e.to_string() }).await;
                    }
                },
                Err(e) => {
                    let _ = tx.send(Msg::DownloadFail { path, error: e.to_string() }).await;
                }
            }
        });
    }

    pub fn do_download_selected(&mut self) {
        let files = self.selected_files();
        if files.is_empty() {
            if let Some(item) = self.current_item().cloned() {
                if item.kind == "file" { self.do_download_item(item); }
            }
        } else {
            for f in files { self.do_download_item(f); }
        }
    }

    pub fn do_copy_url(&mut self) {
        let Some(item) = self.current_item() else { return };
        let url = raw_url(&self.owner, &self.repo, &self.branch, &item.path);
        if try_clipboard(&url) {
            self.status = format!("  ✓ copied: {url}");
        } else {
            self.status = format!("  raw url: {url}");
        }
    }

    pub fn do_wget_cmd(&mut self) {
        let Some(item) = self.current_item() else { return };
        let url = raw_url(&self.owner, &self.repo, &self.branch, &item.path);
        let cmd = format!("wget {url}");
        if try_clipboard(&cmd) {
            self.status = format!("  ✓ copied wget command");
        } else {
            self.status = format!("  wget {url}");
        }
    }

    // ── Message handler ───────────────────────────────────────────────────────

    pub fn handle_msg(&mut self, msg: Msg) {
        self.loading = false;
        match msg {
            Msg::RepoLoaded(info, branches) => {
                let branch  = info.default_branch.clone();
                let full    = info.full_name.clone();
                let stars   = info.stargazers_count;
                let lang    = info.language.clone().unwrap_or_default();
                self.repo_info = Some(info);
                self.branch    = branch.clone();
                self.branches  = branches;

                let bi = self.branches.iter().position(|b| b == &self.branch).unwrap_or(0);
                self.branch_list_state.select(Some(bi));

                self.current_path.clear();
                self.path_stack.clear();
                self.files.clear();
                self.filtered.clear();
                self.selected.clear();
                self.preview = None;
                self.screen  = Screen::Browser;

                let entry = HistoryEntry {
                    owner: self.owner.clone(),
                    repo:  self.repo.clone(),
                    branch: branch.clone(),
                };
                self.history.retain(|h| !(h.owner == entry.owner && h.repo == entry.repo));
                self.history.insert(0, entry);
                self.history.truncate(10);
                self.config.history = self.history.clone();

                let lang_str = if lang.is_empty() { String::new() } else { format!("  {lang}") };
                self.status = format!("  {full}  ⎇ {branch}  ★ {stars}{lang_str}");
                self.do_load_contents(String::new());
            }

            Msg::ContentsLoaded(path, items) => {
                self.current_path = path;
                self.files = items;
                self.search_query.clear();
                self.list_state.select(if self.files.is_empty() { None } else { Some(0) });
                self.rebuild_filter();

                let dirs  = self.files.iter().filter(|f| f.kind == "dir").count();
                let files = self.files.len() - dirs;
                let disp  = if self.current_path.is_empty() { "/".into() }
                             else { format!("/{}", self.current_path) };
                self.status = format!(
                    "  {}/{}{disp}  ·  {dirs} dirs  {files} files",
                    self.owner, self.repo
                );
            }

            Msg::PreviewLoaded(content) => { self.preview = Some(content); }

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

            Msg::ApiError(e) => {
                self.error = Some(e.clone());
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
                    self.preview_scroll = self.preview_scroll.saturating_add(5);
                    false
                }
                KC::Char('k') | KC::Up => {
                    self.preview_scroll = self.preview_scroll.saturating_sub(5);
                    false
                }
                KC::Char('d') => { self.nav_page(10); false }
                KC::Char('u') => { self.nav_page(-10); false }
                _ => false,
            };
        }

        match self.screen.clone() {
            Screen::Home        => self.key_home(code),
            Screen::Browser     => self.key_browser(code),
            Screen::BranchPopup => self.key_branch(code),
            Screen::Help        => { self.screen = Screen::Browser; false }
            Screen::Config      => self.key_config(code),
        }
    }

    fn key_home(&mut self, code: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode as KC;
        match code {
            KC::Char(c) => {
                self.input.insert(self.input_cursor, c);
                self.input_cursor += c.len_utf8();
                self.error = None;
                false
            }
            KC::Backspace => {
                if self.input_cursor > 0 {
                    // Remove char before cursor (handle multi-byte)
                    let mut new_cursor = self.input_cursor;
                    loop {
                        new_cursor -= 1;
                        if self.input.is_char_boundary(new_cursor) { break; }
                    }
                    self.input.remove(new_cursor);
                    self.input_cursor = new_cursor;
                }
                false
            }
            KC::Left  => { if self.input_cursor > 0 { self.input_cursor -= 1; } false }
            KC::Right => {
                if self.input_cursor < self.input.len() { self.input_cursor += 1; }
                false
            }
            KC::Home  => { self.input_cursor = 0; false }
            KC::End   => { self.input_cursor = self.input.len(); false }
            KC::Enter => {
                let s = self.input.trim().to_string();
                if let Some((owner, repo)) = crate::github::parse_url(&s) {
                    self.do_load_repo(owner, repo);
                } else if !s.is_empty() {
                    self.error = Some(format!("not a valid GitHub URL: {s}"));
                }
                false
            }
            KC::Up => {
                // Fill input with most recent history entry
                if let Some(h) = self.history.first() {
                    self.input = format!("{}/{}", h.owner, h.repo);
                    self.input_cursor = self.input.len();
                }
                false
            }
            KC::Char('C') | KC::F(5) => { self.screen = Screen::Config; false }
            KC::Esc | KC::Char('q') => true,
            _ => false,
        }
    }

    fn key_browser(&mut self, code: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode as KC;

        // Search mode eats character input
        if self.search_active {
            match code {
                KC::Char(c) => {
                    self.search_query.push(c);
                    self.rebuild_filter();
                    return false;
                }
                KC::Backspace => {
                    self.search_query.pop();
                    self.rebuild_filter();
                    return false;
                }
                KC::Esc | KC::Enter => {
                    self.search_active = false;
                    if code == KC::Esc {
                        self.search_query.clear();
                        self.rebuild_filter();
                    }
                    return false;
                }
                _ => {}
            }
        }

        match code {
            KC::Char('q') | KC::Char('Q') => {
                if self.path_stack.is_empty() && self.current_path.is_empty() {
                    return true;
                }
                self.path_stack.clear();
                self.do_load_contents(String::new());
                false
            }
            KC::Esc => {
                if self.preview.is_some() {
                    self.preview = None;
                    self.preview_path = None;
                } else if !self.path_stack.is_empty() {
                    self.go_back();
                } else {
                    self.screen = Screen::Home;
                }
                false
            }

            // Navigation
            KC::Up    | KC::Char('k') => { self.nav_up(); false }
            KC::Down  | KC::Char('j') => { self.nav_down(); false }
            KC::Home  | KC::Char('g') => { self.nav_top(); false }
            KC::End   | KC::Char('G') => { self.nav_bottom(); false }
            KC::PageUp   => { self.nav_page(-10); false }
            KC::PageDown => { self.nav_page(10);  false }

            // Enter / navigate
            KC::Enter | KC::Char('l') | KC::Right => {
                if let Some(item) = self.current_item().cloned() {
                    if item.kind == "dir" {
                        self.path_stack.push(self.current_path.clone());
                        let p = item.path.clone();
                        self.do_load_contents(p);
                    } else {
                        self.do_load_preview(item);
                    }
                }
                false
            }

            // Back
            KC::Backspace | KC::Char('h') | KC::Left => {
                if self.preview.is_some() {
                    self.preview = None;
                    self.preview_path = None;
                } else {
                    self.go_back();
                }
                false
            }

            // Selection
            KC::Char(' ') => { self.toggle_select(); false }
            KC::Char('a') => { self.select_all(); false }
            KC::Char('u') => { self.unselect_all(); false }

            // Download
            KC::Char('d') => { self.do_download_selected(); false }
            KC::Char('D') => {
                if let Some(item) = self.current_item().cloned() {
                    if item.kind == "file" { self.do_download_item(item); }
                }
                false
            }

            // Search
            KC::Char('/') => {
                self.search_active = true;
                self.search_query.clear();
                self.rebuild_filter();
                false
            }

            // Branch switcher
            KC::Char('b') | KC::Char('B') => { self.screen = Screen::BranchPopup; false }

            // Refresh
            KC::Char('r') | KC::Char('R') => {
                let p = self.current_path.clone();
                self.do_load_contents(p);
                false
            }

            // Copy URL / wget
            KC::Char('c') => { self.do_copy_url(); false }
            KC::Char('w') => { self.do_wget_cmd(); false }

            // Preview toggle
            KC::Char('p') => {
                if self.preview.is_some() {
                    self.preview = None;
                    self.preview_path = None;
                } else if let Some(item) = self.current_item().cloned() {
                    if item.kind == "file" { self.do_load_preview(item); }
                }
                false
            }

            // Help
            KC::Char('?') | KC::F(1) => { self.screen = Screen::Help; false }

            // Config
            KC::Char('C') => { self.screen = Screen::Config; false }

            // Clear error
            KC::Char('e') => { self.error = None; false }

            _ => false,
        }
    }

    fn key_branch(&mut self, code: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode as KC;
        match code {
            KC::Up | KC::Char('k') => {
                let i = self.branch_list_state.selected().unwrap_or(0);
                self.branch_list_state.select(Some(i.saturating_sub(1)));
                false
            }
            KC::Down | KC::Char('j') => {
                let i = self.branch_list_state.selected().unwrap_or(0);
                if i + 1 < self.branches.len() {
                    self.branch_list_state.select(Some(i + 1));
                }
                false
            }
            KC::Enter => {
                if let Some(i) = self.branch_list_state.selected() {
                    if let Some(b) = self.branches.get(i).cloned() {
                        self.branch = b;
                        self.path_stack.clear();
                        self.screen = Screen::Browser;
                        self.do_load_contents(String::new());
                    }
                }
                false
            }
            KC::Esc | KC::Char('q') | KC::Char('b') => {
                self.screen = Screen::Browser;
                false
            }
            _ => false,
        }
    }

    fn key_config(&mut self, code: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode as KC;
        if self.cfg_editing {
            match code {
                KC::Char(c) => { self.cfg_buf.push(c); false }
                KC::Backspace => { self.cfg_buf.pop(); false }
                KC::Enter => {
                    match self.cfg_field {
                        0 => {
                            self.config.token = if self.cfg_buf.is_empty() {
                                None
                            } else { Some(self.cfg_buf.clone()) };
                            // Rebuild client with new token
                            self.gh = GhClient::new(self.config.token.clone());
                        }
                        1 => {
                            self.config.download_path = if self.cfg_buf.is_empty() {
                                dirs::download_dir()
                            } else {
                                Some(std::path::PathBuf::from(&self.cfg_buf))
                            };
                        }
                        _ => {}
                    }
                    self.config.save();
                    self.cfg_editing = false;
                    self.status = "  ✓ config saved".to_string();
                    false
                }
                KC::Esc => { self.cfg_editing = false; false }
                _ => false,
            }
        } else {
            match code {
                KC::Up | KC::Char('k') => {
                    if self.cfg_field > 0 { self.cfg_field -= 1; }
                    false
                }
                KC::Down | KC::Char('j') => {
                    self.cfg_field = (self.cfg_field + 1).min(1);
                    false
                }
                KC::Enter => {
                    self.cfg_buf = match self.cfg_field {
                        0 => self.config.token.clone().unwrap_or_default(),
                        1 => self.config.download_path.as_ref()
                                 .map(|p| p.display().to_string())
                                 .unwrap_or_default(),
                        _ => String::new(),
                    };
                    self.cfg_editing = true;
                    false
                }
                KC::Esc | KC::Char('q') => {
                    self.screen = if self.repo_info.is_some() { Screen::Browser }
                                  else { Screen::Home };
                    false
                }
                _ => false,
            }
        }
    }
}

// ─── Clipboard helper ─────────────────────────────────────────────────────────

fn try_clipboard(text: &str) -> bool {
    use std::io::Write;
    for cmd in [
        &["xclip", "-selection", "clipboard"][..],
        &["xsel", "--clipboard", "--input"][..],
        &["wl-copy"][..],
        &["pbcopy"][..],
    ] {
        if let Ok(mut child) = std::process::Command::new(cmd[0])
            .args(&cmd[1..])
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if child.wait().map(|s| s.success()).unwrap_or(false) {
                return true;
            }
        }
    }
    false
}
