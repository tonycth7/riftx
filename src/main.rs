mod app;
mod config;
mod github;
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

// ─── CLI args ─────────────────────────────────────────────────────────────────

fn parse_args() -> Option<String> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        std::process::exit(0);
    }

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("riftx {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    if args.first().map(|s| s.as_str()) == Some("config") {
        handle_config_cmd(&args[1..]);
        std::process::exit(0);
    }

    args.into_iter().find(|a| !a.starts_with('-'))
}

fn print_usage() {
    println!(
        r#"riftx {} — explore & extract files from remote repos without cloning

USAGE:
  riftx [URL]               Launch TUI (optionally open a repo directly)
  riftx config set token T  Save your GitHub token (5000 req/hr vs 60)
  riftx config set path  P  Set default download directory
  riftx config list         Show current configuration
  riftx config unset token  Remove saved token
  riftx config unset path   Reset download path to default
  riftx --help              Show this message
  riftx --version           Print version

KEYS (inside TUI):
  j/k ↑↓     navigate       Space  toggle select
  Enter/l    enter dir       a      select all
  h/Bksp     go back         u      unselect all
  d          download sel    D      download current
  p          preview file    c      copy raw URL
  w          copy wget cmd   /      filter files
  b          switch branch   r      refresh dir
  ?          help popup      C      config
  q/Esc      back/quit       Ctrl+C force quit

PROVIDERS (Phase 1):
  GitHub — https://github.com/owner/repo
"#,
        env!("CARGO_PKG_VERSION")
    );
}

fn handle_config_cmd(args: &[String]) {
    let mut cfg = Config::load();
    match args {
        [cmd] if cmd == "list" => {
            let token = cfg.token.as_deref().map(|t| {
                if t.len() > 8 { format!("{}...{}", &t[..4], &t[t.len()-4..]) }
                else { "****".to_string() }
            }).unwrap_or_else(|| "(not set)".to_string());
            let path = cfg.download_path.as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "(default)".to_string());
            println!("token  : {token}");
            println!("path   : {path}");
        }
        [cmd, key, val] if cmd == "set" => {
            match key.as_str() {
                "token" => { cfg.token = Some(val.clone()); println!("✓ token saved"); }
                "path"  => {
                    cfg.download_path = Some(std::path::PathBuf::from(val));
                    println!("✓ path saved");
                }
                other   => eprintln!("unknown key '{other}'  (use: token | path)"),
            }
            cfg.save();
        }
        [cmd, key] if cmd == "unset" => {
            match key.as_str() {
                "token" => { cfg.token = None; println!("✓ token removed"); }
                "path"  => { cfg.download_path = None; println!("✓ path reset to default"); }
                other   => eprintln!("unknown key '{other}'"),
            }
            cfg.save();
        }
        _ => {
            eprintln!("usage: riftx config [set token|path <val>] [unset token|path] [list]");
        }
    }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let url_arg = parse_args();
    let config  = Config::load();

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;
    term.clear()?;

    let result = run(&mut term, config, url_arg).await;

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()?;

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
    Ok(())
}

// ─── Event loop ───────────────────────────────────────────────────────────────

async fn run(
    term:    &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    config:  Config,
    url_arg: Option<String>,
) -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<Msg>(64);
    let mut app = App::new(tx, config);

    if let Some(url) = url_arg {
        if let Some((owner, repo)) = github::parse_url(&url) {
            app.do_load_repo(owner, repo);
        } else {
            app.error = Some(format!("not a valid GitHub URL: {url}"));
        }
    }

    let tick = Duration::from_millis(50);

    loop {
        term.draw(|f| ui::draw(f, &mut app))?;

        if event::poll(tick)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if app.handle_key(key.code, key.modifiers) {
                        app.config.history = app.history.clone();
                        app.config.save();
                        break;
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        while let Ok(msg) = rx.try_recv() {
            app.handle_msg(msg);
        }
    }

    Ok(())
}
