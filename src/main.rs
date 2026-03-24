mod app;
mod config;
mod fuzzy;
mod providers;
mod theme;
mod ui;

use std::time::Duration;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

use app::{App, Msg};
use config::Config;

struct Args {
    url:        Option<String>,
    ext_filter: Option<String>,
    theme:      Option<String>,
}

fn parse_args() -> Args {
    let raw: Vec<String> = std::env::args().skip(1).collect();

    if raw.iter().any(|a| a == "--help" || a == "-h") { print_usage(); std::process::exit(0); }
    if raw.iter().any(|a| a == "--version" || a == "-V") {
        println!("riftx {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }
    if raw.first().map(|s| s.as_str()) == Some("config") {
        handle_config_cmd(&raw[1..]); std::process::exit(0);
    }

    let mut url:    Option<String> = None;
    let mut ext:    Option<String> = None;
    let mut theme:  Option<String> = None;
    let mut it = raw.iter().peekable();

    while let Some(a) = it.next() {
        match a.as_str() {
            "browse" | "get" | "sync" => { url = it.next().cloned(); }
            "--ext"   => { ext   = it.next().cloned(); }
            "--theme" => { theme = it.next().cloned(); }
            s if s.starts_with("--ext=")   => { ext   = Some(s[6..].to_string()); }
            s if s.starts_with("--theme=") => { theme = Some(s[8..].to_string()); }
            s if !s.starts_with('-')       => { url   = Some(s.to_string()); }
            _ => {}
        }
    }
    Args { url, ext_filter: ext, theme }
}

fn print_usage() {
    println!(r#"riftx {} — explore & extract files from remote repos

PROVIDERS:  GitHub · GitLab · Codeberg · Gitea (self-hosted)

USAGE:
  riftx [URL]                   Launch TUI
  riftx browse <URL>            Browse a repo
  riftx get    <URL>            Open repo (alias)
  riftx config set   <key> <v> Save config value
  riftx config unset <key>     Remove config value
  riftx config list             Show config
  riftx --theme <n>             Override theme at launch
  riftx --ext   <ext>           Pre-filter by extension

THEMES:  amber  dracula  nord  gruvbox  catppuccin  skyblue  tokyonight  ayu

KEYS (TUI):
  j/k ↑↓    navigate     Space  toggle select
  Enter/l    enter dir    a/u/i  all/none/invert
  d          plan popup   D      download now
  p          preview      c/w    copy URL/wget
  /  %  \   search name/ext/path
  b          branches     r      refresh
  T          cycle theme  C      config
  S          cycle sort   f      size filter
  n          new repo     m      bookmark
  ?          help         q/Esc  quit
"#, env!("CARGO_PKG_VERSION"));
}

fn handle_config_cmd(args: &[String]) {
    let mut cfg = Config::load();
    match args {
        [cmd] if cmd == "list" => {
            println!("[core]");
            println!("  theme        = {}", cfg.core.theme.as_str());
            println!("  parallel     = {}", cfg.core.parallel);
            println!("  download     = {}", cfg.core.download_path.as_ref()
                .map(|p| p.display().to_string()).unwrap_or_else(|| "(default)".into()));
            println!("\n[auth]");
            for (k, v) in [
                ("github_token",   &cfg.auth.github_token),
                ("gitlab_token",   &cfg.auth.gitlab_token),
                ("codeberg_token", &cfg.auth.codeberg_token),
                ("gitea_token",    &cfg.auth.gitea_token),
                ("gitea_url",      &cfg.auth.gitea_url),
            ] {
                let display = v.as_deref().map(|t| {
                    if k.ends_with("token") && t.len() > 8 {
                        format!("{}…{}", &t[..4], &t[t.len()-4..])
                    } else { t.to_string() }
                }).unwrap_or_else(|| "(not set)".into());
                println!("  {k:<20} = {display}");
            }
        }
        [cmd, key, val] if cmd == "set" => {
            match key.as_str() {
                "github_token"   => cfg.auth.github_token   = Some(val.clone()),
                "gitlab_token"   => cfg.auth.gitlab_token   = Some(val.clone()),
                "codeberg_token" => cfg.auth.codeberg_token = Some(val.clone()),
                "gitea_token"    => cfg.auth.gitea_token    = Some(val.clone()),
                "gitea_url"      => cfg.auth.gitea_url      = Some(val.clone()),
                "theme"          => cfg.core.theme          = config::ThemeName::from_str(val),
                "parallel"       => if let Ok(n) = val.parse::<u8>() { cfg.core.parallel = n; },
                "path"           => cfg.core.download_path  = Some(std::path::PathBuf::from(val)),
                other            => { eprintln!("unknown key: {other}"); return; }
            }
            cfg.save();
            println!("✓ {key} = {val}");
        }
        [cmd, key] if cmd == "unset" => {
            match key.as_str() {
                "github_token"   => cfg.auth.github_token   = None,
                "gitlab_token"   => cfg.auth.gitlab_token   = None,
                "codeberg_token" => cfg.auth.codeberg_token = None,
                "gitea_token"    => cfg.auth.gitea_token    = None,
                "gitea_url"      => cfg.auth.gitea_url      = None,
                "path"           => cfg.core.download_path  = None,
                other            => { eprintln!("unknown key: {other}"); return; }
            }
            cfg.save();
            println!("✓ {key} unset");
        }
        _ => eprintln!("usage: riftx config [list | set <key> <val> | unset <key>]"),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args   = parse_args();
    let mut cfg = Config::load();

    if let Some(ref t) = args.theme {
        cfg.core.theme = config::ThemeName::from_str(t);
    }

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;
    term.clear()?;

    let result = run(&mut term, cfg, args).await;

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()?;

    if let Err(e) = result { eprintln!("error: {e}"); std::process::exit(1); }
    Ok(())
}

async fn run(
    term:   &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    config: Config,
    args:   Args,
) -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<Msg>(64);
    let mut app = App::new(tx, config);

    if let Some(ext) = args.ext_filter { app.ext_filter = Some(ext); }

    if let Some(url) = args.url {
        let gitea_inst = app.config.auth.gitea_url.clone();
        if let Some((kind, owner, repo, inst)) =
            providers::parse_url(&url, gitea_inst.as_deref())
        {
            app.do_load_repo(kind, owner, repo, inst);
        } else {
            app.error = Some(format!("invalid URL: {url}"));
        }
    }

    let tick = Duration::from_millis(50);
    loop {
        term.draw(|f| ui::draw(f, &mut app))?;

        if event::poll(tick)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if app.handle_key(key.code, key.modifiers) {
                        app.config.save();
                        break;
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
        while let Ok(msg) = rx.try_recv() { app.handle_msg(msg); }
    }
    Ok(())
}
