use reqwest::Client;
use serde::Deserialize;

// ─── API types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct RepoInfo {
    pub full_name:        String,
    pub description:      Option<String>,
    pub default_branch:   String,
    pub stargazers_count: u64,
    pub forks_count:      u64,
    pub private:          bool,
    pub language:         Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhItem {
    pub name:         String,
    pub path:         String,
    #[serde(rename = "type")]
    pub kind:         String,   // "file" | "dir"
    pub size:         Option<u64>,
    pub download_url: Option<String>,
    pub sha:          String,
}

#[derive(Debug, Clone, Deserialize)]
struct BranchResp {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitInfo {
    pub sha: String,
    pub commit: CommitDetail,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitDetail {
    pub message: String,
    pub author:  CommitAuthor,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitAuthor {
    pub name: String,
    pub date: String,
}

// ─── Client ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct GhClient {
    client: Client,
    token:  Option<String>,
}

impl GhClient {
    pub fn new(token: Option<String>) -> Self {
        let client = Client::builder()
            .user_agent("riftx/0.0.3 (rust tui)")
            .build()
            .expect("http client");
        Self { client, token }
    }

    fn auth(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("token {t}"))
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> anyhow::Result<T> {
        let mut req = self.client.get(url)
            .header("Accept", "application/vnd.github.v3+json");
        if let Some(auth) = self.auth() {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            let val: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
            return Err(anyhow::anyhow!(
                "{} (HTTP {})",
                val["message"].as_str().unwrap_or("unknown error"),
                status.as_u16()
            ));
        }
        Ok(resp.json().await?)
    }

    pub async fn get_repo(&self, owner: &str, repo: &str) -> anyhow::Result<RepoInfo> {
        self.get_json(&format!("https://api.github.com/repos/{owner}/{repo}")).await
    }

    pub async fn get_contents(
        &self, owner: &str, repo: &str, path: &str, branch: &str,
    ) -> anyhow::Result<Vec<GhItem>> {
        let url = format!(
            "https://api.github.com/repos/{owner}/{repo}/contents/{path}?ref={branch}"
        );
        let mut items: Vec<GhItem> = self.get_json(&url).await?;
        // dirs first, then alpha
        items.sort_by(|a, b| match (a.kind.as_str(), b.kind.as_str()) {
            ("dir", "file") => std::cmp::Ordering::Less,
            ("file", "dir") => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });
        Ok(items)
    }

    pub async fn get_branches(&self, owner: &str, repo: &str) -> anyhow::Result<Vec<String>> {
        let resp: Vec<BranchResp> = self.get_json(&format!(
            "https://api.github.com/repos/{owner}/{repo}/branches?per_page=100"
        )).await?;
        Ok(resp.into_iter().map(|b| b.name).collect())
    }

    pub async fn get_text(&self, url: &str) -> anyhow::Result<String> {
        let mut req = self.client.get(url);
        if let Some(auth) = self.auth() { req = req.header("Authorization", auth); }
        Ok(req.send().await?.text().await?)
    }

    pub async fn get_bytes(&self, url: &str) -> anyhow::Result<bytes::Bytes> {
        let mut req = self.client.get(url);
        if let Some(auth) = self.auth() { req = req.header("Authorization", auth); }
        Ok(req.send().await?.bytes().await?)
    }

    pub async fn get_latest_commit(
        &self, owner: &str, repo: &str, branch: &str, path: &str,
    ) -> anyhow::Result<CommitInfo> {
        self.get_json(&format!(
            "https://api.github.com/repos/{owner}/{repo}/commits?sha={branch}&path={path}&per_page=1"
        ))
        .await
        .and_then(|mut v: Vec<CommitInfo>| {
            v.pop().ok_or_else(|| anyhow::anyhow!("no commits found"))
        })
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

pub fn parse_url(input: &str) -> Option<(String, String)> {
    let s = input.trim().trim_end_matches('/');
    let s = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"))
        .or_else(|| s.strip_prefix("github.com/"))
        .unwrap_or(s);
    let parts: Vec<&str> = s.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() >= 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

pub fn raw_url(owner: &str, repo: &str, branch: &str, path: &str) -> String {
    format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{path}")
}

pub fn fmt_size(bytes: u64) -> String {
    if bytes < 1_024 {
        format!("{bytes}B")
    } else if bytes < 1_024 * 1_024 {
        format!("{:.1}K", bytes as f64 / 1_024.0)
    } else {
        format!("{:.1}M", bytes as f64 / (1_024.0 * 1_024.0))
    }
}

pub fn is_binary_ext(name: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico"
        | "pdf" | "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar"
        | "exe" | "bin" | "wasm" | "so" | "dylib" | "dll" | "a" | "o"
        | "ttf" | "otf" | "woff" | "woff2"
        | "mp4" | "mov" | "avi" | "mkv" | "webm"
        | "mp3" | "wav" | "ogg" | "flac" | "aac"
        | "db" | "sqlite" | "sqlite3"
    )
}
