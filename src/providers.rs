use reqwest::Client;
use serde::Deserialize;
use anyhow::Result;

// ─── Shared Node type ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind { Dir, File }

#[derive(Debug, Clone)]
pub struct Node {
    pub name:         String,
    pub path:         String,
    pub kind:         NodeKind,
    pub size:         Option<u64>,
    pub download_url: Option<String>,
    #[allow(dead_code)]
    pub sha:          String,
}

// ─── Repo metadata ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RepoMeta {
    pub full_name:   String,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub def_branch:  String,
    pub stars:       u64,
    #[allow(dead_code)]
    pub forks:       u64,
    pub private:     bool,
    pub language:    Option<String>,
}

// ─── Provider kind (for auto-detect + display) ───────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProviderKind {
    GitHub,
    GitLab,
    Codeberg,
    Gitea,    // self-hosted
}

impl ProviderKind {
    #[allow(dead_code)]
    pub fn detect(url: &str) -> Option<(Self, Option<String>)> {
        let u = url.to_lowercase();
        if u.contains("github.com")   { return Some((Self::GitHub,   None)); }
        if u.contains("gitlab.com")   { return Some((Self::GitLab,   None)); }
        if u.contains("codeberg.org") { return Some((Self::Codeberg, None)); }
        None
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::GitHub   => "GitHub",
            Self::GitLab   => "GitLab",
            Self::Codeberg => "Codeberg",
            Self::Gitea    => "Gitea",
        }
    }

    pub fn badge(self) -> &'static str {
        match self {
            Self::GitHub   => "GH",
            Self::GitLab   => "GL",
            Self::Codeberg => "CB",
            Self::Gitea    => "GT",
        }
    }
}

// ─── Common HTTP client wrapper ───────────────────────────────────────────────

#[derive(Clone)]
pub struct ApiClient {
    pub client:   Client,
    pub kind:     ProviderKind,
    pub token:    Option<String>,
    pub base_url: String,
}

impl ApiClient {
    pub fn new(kind: ProviderKind, token: Option<String>, instance: Option<&str>) -> Self {
        let base_url = match kind {
            ProviderKind::GitHub   => "https://api.github.com".into(),
            ProviderKind::GitLab   => "https://gitlab.com/api/v4".into(),
            ProviderKind::Codeberg => "https://codeberg.org/api/v1".into(),
            ProviderKind::Gitea    => {
                let inst = instance.unwrap_or("https://gitea.com");
                format!("{}/api/v1", inst.trim_end_matches('/'))
            }
        };
        let client = Client::builder()
            .user_agent("riftx/0.0.7 (rust tui)")
            .build()
            .expect("http client");
        Self { client, kind, token, base_url }
    }

    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| match self.kind {
            ProviderKind::GitHub   => format!("token {t}"),
            ProviderKind::GitLab   => format!("Bearer {t}"),
            ProviderKind::Codeberg |
            ProviderKind::Gitea    => format!("token {t}"),
        })
    }

    pub async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let mut req = self.client.get(url);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }
        if let ProviderKind::GitHub = self.kind {
            req = req.header("Accept", "application/vnd.github.v3+json");
        }
        let resp   = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v["message"].as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
            return Err(anyhow::anyhow!("{msg} (HTTP {})", status.as_u16()));
        }
        Ok(resp.json().await?)
    }

    pub async fn get_text(&self, url: &str) -> Result<String> {
        let mut req = self.client.get(url);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }
        Ok(req.send().await?.text().await?)
    }

    pub async fn get_bytes(&self, url: &str) -> Result<bytes::Bytes> {
        let mut req = self.client.get(url);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }
        Ok(req.send().await?.bytes().await?)
    }
}

// ─── GitHub ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)] struct GhRepo {
    full_name: String, description: Option<String>,
    default_branch: String, stargazers_count: u64,
    forks_count: u64, private: bool, language: Option<String>,
}
#[derive(Deserialize)] struct GhItem {
    name: String, path: String,
    #[serde(rename = "type")] kind: String,
    size: Option<u64>, download_url: Option<String>,
    sha: String,
}
#[derive(Deserialize)] struct GhBranch { name: String }

pub async fn gh_get_repo(c: &ApiClient, owner: &str, repo: &str) -> Result<RepoMeta> {
    let r: GhRepo = c.get_json(&format!("{}/repos/{owner}/{repo}", c.base_url)).await?;
    Ok(RepoMeta {
        full_name: r.full_name, description: r.description,
        def_branch: r.default_branch, stars: r.stargazers_count,
        forks: r.forks_count, private: r.private, language: r.language,
    })
}

pub async fn gh_list_contents(
    c: &ApiClient, owner: &str, repo: &str, path: &str, branch: &str,
) -> Result<Vec<Node>> {
    let url = format!("{}/repos/{owner}/{repo}/contents/{path}?ref={branch}", c.base_url);
    let items: Vec<GhItem> = c.get_json(&url).await?;
    let mut nodes: Vec<Node> = items.into_iter().map(|i| Node {
        name: i.name, path: i.path,
        kind: if i.kind == "dir" { NodeKind::Dir } else { NodeKind::File },
        size: i.size, download_url: i.download_url, sha: i.sha,
    }).collect();
    sort_nodes(&mut nodes);
    Ok(nodes)
}

pub async fn gh_list_branches(c: &ApiClient, owner: &str, repo: &str) -> Result<Vec<String>> {
    let bs: Vec<GhBranch> = c.get_json(
        &format!("{}/repos/{owner}/{repo}/branches?per_page=100", c.base_url)
    ).await?;
    Ok(bs.into_iter().map(|b| b.name).collect())
}

// ─── GitLab ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)] struct GlProject {
    name_with_namespace: String, description: Option<String>,
    default_branch: Option<String>, star_count: u64,
    forks_count: u64, visibility: String,
}
#[derive(Deserialize)] struct GlTreeItem {
    name: String, path: String,
    #[serde(rename = "type")] kind: String,
    id: String,
}
#[derive(Deserialize)] struct GlBranch { name: String }
#[allow(dead_code)]
#[derive(Deserialize)] struct GlFile { size: Option<u64> }

fn gl_encode(owner: &str, repo: &str) -> String {
    format!("{owner}%2F{repo}")
}

pub async fn gl_get_repo(c: &ApiClient, owner: &str, repo: &str) -> Result<RepoMeta> {
    let id = gl_encode(owner, repo);
    let r: GlProject = c.get_json(&format!("{}/projects/{id}", c.base_url)).await?;
    Ok(RepoMeta {
        full_name:   r.name_with_namespace,
        description: r.description,
        def_branch:  r.default_branch.unwrap_or_else(|| "main".into()),
        stars:       r.star_count,
        forks:       r.forks_count,
        private:     r.visibility == "private",
        language:    None,
    })
}

pub async fn gl_list_contents(
    c: &ApiClient, owner: &str, repo: &str, path: &str, branch: &str,
) -> Result<Vec<Node>> {
    let id   = gl_encode(owner, repo);
    let path_param = if path.is_empty() { "".into() } else { format!("&path={path}") };
    let url  = format!(
        "{}/projects/{id}/repository/tree?ref={branch}&per_page=100{path_param}",
        c.base_url
    );
    let items: Vec<GlTreeItem> = c.get_json(&url).await?;

    let mut nodes: Vec<Node> = items.into_iter().map(|i| {
        let is_dir  = i.kind == "tree";
        let dl_url  = if is_dir { None } else {
            Some(format!(
                "{}/projects/{id}/repository/files/{}/raw?ref={branch}",
                c.base_url,
                urlenc(&i.path),
            ))
        };
        Node {
            name: i.name, path: i.path,
            kind: if is_dir { NodeKind::Dir } else { NodeKind::File },
            size: None,
            download_url: dl_url, sha: i.id,
        }
    }).collect();
    sort_nodes(&mut nodes);
    Ok(nodes)
}

pub async fn gl_list_branches(c: &ApiClient, owner: &str, repo: &str) -> Result<Vec<String>> {
    let id = gl_encode(owner, repo);
    let bs: Vec<GlBranch> = c.get_json(
        &format!("{}/projects/{id}/repository/branches?per_page=100", c.base_url)
    ).await?;
    Ok(bs.into_iter().map(|b| b.name).collect())
}

fn urlenc(s: &str) -> String {
    s.replace('/', "%2F")
}

// ─── Gitea / Codeberg (same API, different base URLs) ─────────────────────────

#[derive(Deserialize)] struct GtRepo {
    full_name: String, description: Option<String>,
    default_branch: String, stars_count: u64,
    forks_count: u64, private: bool,
    language: Option<String>,
}
#[derive(Deserialize)] struct GtItem {
    name: String, path: String,
    #[serde(rename = "type")] kind: String,
    size: Option<u64>,
    download_url: Option<String>,
    sha: String,
}
#[derive(Deserialize)] struct GtBranch { name: String }

pub async fn gt_get_repo(c: &ApiClient, owner: &str, repo: &str) -> Result<RepoMeta> {
    let r: GtRepo = c.get_json(&format!("{}/repos/{owner}/{repo}", c.base_url)).await?;
    Ok(RepoMeta {
        full_name: r.full_name, description: r.description,
        def_branch: r.default_branch, stars: r.stars_count,
        forks: r.forks_count, private: r.private, language: r.language,
    })
}

pub async fn gt_list_contents(
    c: &ApiClient, owner: &str, repo: &str, path: &str, branch: &str,
) -> Result<Vec<Node>> {
    let url = format!("{}/repos/{owner}/{repo}/contents/{path}?ref={branch}", c.base_url);
    let items: Vec<GtItem> = c.get_json(&url).await?;
    let mut nodes: Vec<Node> = items.into_iter().map(|i| Node {
        name: i.name, path: i.path,
        kind: if i.kind == "dir" { NodeKind::Dir } else { NodeKind::File },
        size: i.size, download_url: i.download_url, sha: i.sha,
    }).collect();
    sort_nodes(&mut nodes);
    Ok(nodes)
}

pub async fn gt_list_branches(c: &ApiClient, owner: &str, repo: &str) -> Result<Vec<String>> {
    let bs: Vec<GtBranch> = c.get_json(
        &format!("{}/repos/{owner}/{repo}/branches?limit=100", c.base_url)
    ).await?;
    Ok(bs.into_iter().map(|b| b.name).collect())
}

// ─── Unified dispatch ─────────────────────────────────────────────────────────

pub async fn get_repo(c: &ApiClient, owner: &str, repo: &str) -> Result<RepoMeta> {
    match c.kind {
        ProviderKind::GitHub            => gh_get_repo(c, owner, repo).await,
        ProviderKind::GitLab            => gl_get_repo(c, owner, repo).await,
        ProviderKind::Codeberg |
        ProviderKind::Gitea             => gt_get_repo(c, owner, repo).await,
    }
}

pub async fn list_contents(
    c: &ApiClient, owner: &str, repo: &str, path: &str, branch: &str,
) -> Result<Vec<Node>> {
    match c.kind {
        ProviderKind::GitHub            => gh_list_contents(c, owner, repo, path, branch).await,
        ProviderKind::GitLab            => gl_list_contents(c, owner, repo, path, branch).await,
        ProviderKind::Codeberg |
        ProviderKind::Gitea             => gt_list_contents(c, owner, repo, path, branch).await,
    }
}

pub async fn list_branches(c: &ApiClient, owner: &str, repo: &str) -> Result<Vec<String>> {
    match c.kind {
        ProviderKind::GitHub            => gh_list_branches(c, owner, repo).await,
        ProviderKind::GitLab            => gl_list_branches(c, owner, repo).await,
        ProviderKind::Codeberg |
        ProviderKind::Gitea             => gt_list_branches(c, owner, repo).await,
    }
}

// ─── URL parsing ──────────────────────────────────────────────────────────────

pub fn parse_url(input: &str, gitea_instance: Option<&str>)
    -> Option<(ProviderKind, String, String, Option<String>)>
{
    let s = input.trim().trim_end_matches('/');

    for (host, kind) in [
        ("github.com",   ProviderKind::GitHub),
        ("gitlab.com",   ProviderKind::GitLab),
        ("codeberg.org", ProviderKind::Codeberg),
    ] {
        if let Some(rest) = extract_after_host(s, host) {
            let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty()).collect();
            if parts.len() >= 2 {
                return Some((kind, parts[0].into(), parts[1].into(), None));
            }
        }
    }

    if let Some(inst) = gitea_instance {
        let inst_clean = inst.trim_end_matches('/');
        if let Some(rest) = extract_after_host(s, inst_clean) {
            let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty()).collect();
            if parts.len() >= 2 {
                return Some((ProviderKind::Gitea, parts[0].into(), parts[1].into(),
                             Some(inst_clean.into())));
            }
        }
    }

    let parts: Vec<&str> = s.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() >= 2 && !parts[0].contains('.') {
        return Some((ProviderKind::GitHub, parts[0].into(), parts[1].into(), None));
    }

    None
}

fn extract_after_host<'a>(s: &'a str, host: &str) -> Option<&'a str> {
    for prefix in &["https://", "http://", ""] {
        let pattern = format!("{prefix}{host}");
        if let Some(idx) = s.find(&pattern) {
            return Some(&s[idx + pattern.len()..]);
        }
    }
    None
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

pub fn raw_url(kind: ProviderKind, instance: Option<&str>,
               owner: &str, repo: &str, branch: &str, path: &str) -> String {
    match kind {
        ProviderKind::GitHub =>
            format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{path}"),
        ProviderKind::GitLab =>
            format!("https://gitlab.com/{owner}/{repo}/-/raw/{branch}/{path}"),
        ProviderKind::Codeberg =>
            format!("https://codeberg.org/{owner}/{repo}/raw/branch/{branch}/{path}"),
        ProviderKind::Gitea => {
            let base = instance.unwrap_or("https://gitea.com");
            format!("{base}/{owner}/{repo}/raw/branch/{branch}/{path}")
        }
    }
}

pub fn fmt_size(bytes: u64) -> String {
    if      bytes < 1_024         { format!("{bytes}B") }
    else if bytes < 1_024 * 1_024 { format!("{:.1}K", bytes as f64 / 1_024.0) }
    else                           { format!("{:.1}M", bytes as f64 / (1_024.0 * 1_024.0)) }
}

pub fn is_binary_ext(name: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    matches!(ext.as_str(),
        "png"|"jpg"|"jpeg"|"gif"|"webp"|"bmp"|"ico"|"pdf"
        |"zip"|"tar"|"gz"|"bz2"|"xz"|"7z"|"rar"
        |"exe"|"bin"|"wasm"|"so"|"dylib"|"dll"|"a"|"o"
        |"ttf"|"otf"|"woff"|"woff2"
        |"mp4"|"mov"|"avi"|"mkv"|"webm"
        |"mp3"|"wav"|"ogg"|"flac"|"aac"
        |"db"|"sqlite"|"sqlite3"
    )
}

pub fn file_ext(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or("")
}

fn sort_nodes(nodes: &mut Vec<Node>) {
    nodes.sort_by(|a, b| {
        match (&a.kind, &b.kind) {
            (NodeKind::Dir,  NodeKind::File) => std::cmp::Ordering::Less,
            (NodeKind::File, NodeKind::Dir)  => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });
}
