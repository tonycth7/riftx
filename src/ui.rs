use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph,
        Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};

use crate::app::{App, Screen};
use crate::github::{fmt_size, GhItem};

// ─── Palette ─────────────────────────────────────────────────────────────────

const AMBER:   Color = Color::Rgb(245, 158,  11);
const AMBER2:  Color = Color::Rgb(180, 110,   5);
const AMBER3:  Color = Color::Rgb( 80,  48,   2);
const BRIGHT:  Color = Color::Rgb(226, 232, 240);
const MID:     Color = Color::Rgb(148, 163, 184);
const DIM:     Color = Color::Rgb( 71,  71,  90);
const VDIM:    Color = Color::Rgb( 30,  30,  46);
const BG:      Color = Color::Rgb( 11,  11,  14);
const BG2:     Color = Color::Rgb( 16,  16,  22);
const BG3:     Color = Color::Rgb( 22,  22,  30);
const BG_SEL:  Color = Color::Rgb( 18,  24,  42);
const GREEN:   Color = Color::Rgb( 74, 222, 128);
const BLUE:    Color = Color::Rgb( 96, 165, 250);
const RED:     Color = Color::Rgb(248, 113, 113);
const PURPLE:  Color = Color::Rgb(167, 139, 250);
const CYAN:    Color = Color::Rgb(103, 232, 249);
const ORANGE:  Color = Color::Rgb(249, 115,  22);
const YELLOW:  Color = Color::Rgb(250, 204,  21);
const TEAL:    Color = Color::Rgb( 52, 211, 153);
const PINK:    Color = Color::Rgb(244, 114, 182);

// ─── File icons ──────────────────────────────────────────────────────────────

fn file_icon(name: &str, is_dir: bool) -> (&'static str, Color) {
    if is_dir { return ("▸", AMBER); }
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    // Special filenames first
    let lower = name.to_lowercase();
    if lower == "dockerfile" { return ("■", CYAN); }
    if lower.starts_with("makefile") { return ("■", YELLOW); }
    if lower == ".gitignore" || lower == ".gitattributes" { return ("■", DIM); }
    if lower == "license" || lower == "licence" { return ("≡", MID); }
    if lower == "readme.md" || lower == "readme" { return ("≡", BLUE); }

    match ext.as_str() {
        "rs"                        => ("●", ORANGE),
        "js" | "mjs" | "cjs"       => ("◆", YELLOW),
        "ts"                        => ("◆", BLUE),
        "tsx" | "jsx"               => ("◆", CYAN),
        "py" | "pyw"                => ("◆", GREEN),
        "go"                        => ("◆", CYAN),
        "rb"                        => ("◆", RED),
        "java" | "kt" | "kts"       => ("◆", ORANGE),
        "cpp"|"cc"|"cxx"|"c"|"h"|"hpp" => ("◆", PURPLE),
        "swift"                     => ("◆", ORANGE),
        "dart"                      => ("◆", BLUE),
        "zig"                       => ("◆", AMBER),
        "ex" | "exs"                => ("◆", PURPLE),
        "hs" | "lhs"                => ("◆", PURPLE),
        "lua"                       => ("◆", BLUE),
        "php"                       => ("◆", PURPLE),
        "cs"                        => ("◆", PURPLE),
        "scala" | "sbt"             => ("◆", RED),
        "clj" | "cljs"              => ("◆", GREEN),
        "ml" | "mli"                => ("◆", ORANGE),
        "nim"                       => ("◆", YELLOW),
        "v"                         => ("◆", BLUE),
        "md" | "mdx" | "markdown"   => ("≡", MID),
        "txt" | "text" | "rst"      => ("≡", DIM),
        "json" | "json5" | "jsonc"  => ("{", YELLOW),
        "yaml" | "yml"              => ("{", GREEN),
        "toml"                      => ("{", ORANGE),
        "xml"                       => ("<", MID),
        "html" | "htm"              => ("<", ORANGE),
        "css" | "scss" | "sass"     => ("{", BLUE),
        "svg"                       => ("<", TEAL),
        "sh" | "bash" | "zsh" | "fish" => ("$", GREEN),
        "ps1" | "psm1"              => ("$", BLUE),
        "env"                       => ("#", GREEN),
        "lock"                      => ("-", DIM),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" => ("□", TEAL),
        "pdf"                       => ("□", RED),
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" => ("□", MID),
        "mp4" | "mov" | "mkv" | "webm" => ("▷", MID),
        "mp3" | "wav" | "ogg" | "flac" => ("♪", MID),
        "wasm"                      => ("■", PURPLE),
        "proto"                     => ("◈", CYAN),
        "sql"                       => ("◈", BLUE),
        "nix"                       => ("◈", BLUE),
        "vim" | "nvim" | "lua"      => ("◈", GREEN),
        _                           => ("·", DIM),
    }
}

// ─── Public entry ─────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    // Base background
    f.render_widget(Block::default().style(Style::default().bg(BG)), area);

    match app.screen {
        Screen::Home => draw_home(f, app, area),
        Screen::Browser | Screen::BranchPopup | Screen::Help => {
            draw_browser(f, app, area);
            match app.screen {
                Screen::BranchPopup => draw_branch_popup(f, app, area),
                Screen::Help        => draw_help_popup(f, area),
                _ => {}
            }
        }
        Screen::Config => draw_config(f, app, area),
    }
}

// ─── Home screen ─────────────────────────────────────────────────────────────

const LOGO: &[&str] = &[
    " ██████╗ ██╗███████╗████████╗██╗  ██╗",
    " ██╔══██╗██║██╔════╝╚══██╔══╝╚██╗██╔╝",
    " ██████╔╝██║█████╗     ██║    ╚███╔╝ ",
    " ██╔══██╗██║██╔══╝     ██║    ██╔██╗ ",
    " ██║  ██║██║██║        ██║   ██╔╝╚██╗",
    " ╚═╝  ╚═╝╚═╝╚═╝        ╚═╝   ╚═╝  ╚═╝",
];

fn draw_home(f: &mut Frame, app: &App, area: Rect) {
    let logo_h   = LOGO.len() as u16;
    let hist_cnt = app.history.len().min(6) as u16;
    let total_h  = logo_h + 2 + 3 + 2 + (if hist_cnt > 0 { hist_cnt + 2 } else { 0 });
    let top_pad  = area.height.saturating_sub(total_h) / 2;

    // Logo
    for (i, line) in LOGO.iter().enumerate() {
        let y = top_pad + i as u16;
        if y >= area.height { break; }
        let color = match i {
            0 | 1 => AMBER,
            2 | 3 => AMBER2,
            _     => AMBER3,
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(*line, Style::default().fg(color))))
                .alignment(Alignment::Center),
            Rect { x: 0, y, width: area.width, height: 1 },
        );
    }

    // Tagline
    let tag_y = top_pad + logo_h;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("explore & extract from remote repos  ", Style::default().fg(DIM)),
            Span::styled("no clone needed", Style::default().fg(VDIM)),
        ])).alignment(Alignment::Center),
        Rect { x: 0, y: tag_y, width: area.width, height: 1 },
    );

    // Input box — centered, max 64 wide
    let box_w = 64u16.min(area.width.saturating_sub(4));
    let box_x = (area.width - box_w) / 2;
    let box_y = tag_y + 2;
    let border_col = if app.input.is_empty() { DIM } else { AMBER };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_col))
        .title(Line::from(vec![
            Span::styled(" github.com/", Style::default().fg(DIM)),
        ]));
    let inner = block.inner(Rect { x: box_x, y: box_y, width: box_w, height: 3 });
    f.render_widget(block, Rect { x: box_x, y: box_y, width: box_w, height: 3 });

    let input_line = if app.input.is_empty() {
        Line::from(Span::styled(
            "owner/repo  or  https://github.com/owner/repo",
            Style::default().fg(Color::Rgb(35, 35, 50)),
        ))
    } else {
        Line::from(Span::styled(&app.input, Style::default().fg(BRIGHT)))
    };
    f.render_widget(Paragraph::new(input_line), inner);

    if !app.input.is_empty() {
        let cx = inner.x + (app.input_cursor as u16).min(inner.width.saturating_sub(1));
        f.set_cursor_position((cx, inner.y));
    }

    // Hint row
    let hint_y = box_y + 3;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            key_span("Enter"), hint_span(" load  "),
            key_span("↑"),     hint_span(" history  "),
            key_span("C"),     hint_span(" config  "),
            key_span("q"),     hint_span(" quit"),
        ])).alignment(Alignment::Center),
        Rect { x: 0, y: hint_y, width: area.width, height: 1 },
    );

    // Recent history
    if !app.history.is_empty() {
        let hist_y = hint_y + 2;
        f.render_widget(
            Paragraph::new(Line::from(Span::styled("  RECENT", Style::default().fg(DIM).add_modifier(Modifier::DIM))))
                .alignment(Alignment::Center),
            Rect { x: 0, y: hist_y, width: area.width, height: 1 },
        );
        for (i, h) in app.history.iter().enumerate().take(6) {
            let y = hist_y + 1 + i as u16;
            if y >= area.height { break; }
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(&h.owner,  Style::default().fg(AMBER2)),
                    Span::styled("/",       Style::default().fg(VDIM)),
                    Span::styled(&h.repo,   Style::default().fg(MID)),
                    Span::styled(format!("  ⎇ {}", h.branch), Style::default().fg(VDIM)),
                ])).alignment(Alignment::Center),
                Rect { x: 0, y, width: area.width, height: 1 },
            );
        }
    }

    // Error
    if let Some(ref err) = app.error {
        let y = area.height.saturating_sub(2);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  ✗  {err}"),
                Style::default().fg(RED),
            ))),
            Rect { x: 0, y, width: area.width, height: 1 },
        );
    }

    // Loading spinner (shown in top-right)
    if app.loading {
        let spinner = "◌";
        let x = area.width.saturating_sub(4);
        f.render_widget(
            Paragraph::new(Span::styled(spinner, Style::default().fg(AMBER))),
            Rect { x, y: 0, width: 3, height: 1 },
        );
    }
}

// ─── Browser screen ───────────────────────────────────────────────────────────

fn draw_browser(f: &mut Frame, app: &mut App, area: Rect) {
    // Layout: [top_bar][breadcrumb][content][bottom_bar]
    let vchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // top bar
            Constraint::Length(1),  // breadcrumb
            Constraint::Min(1),     // content
            Constraint::Length(1),  // bottom bar
        ])
        .split(area);

    draw_top_bar(f, app, vchunks[0]);
    draw_breadcrumb(f, app, vchunks[1]);
    draw_content(f, app, vchunks[2]);
    draw_bottom_bar(f, app, vchunks[3]);
}

fn draw_top_bar(f: &mut Frame, app: &App, area: Rect) {
    let mut spans: Vec<Span> = vec![
        Span::raw(" "),
        Span::styled("rift", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
        Span::styled("x",    Style::default().fg(MID)),
        Span::raw("  "),
    ];

    if !app.owner.is_empty() {
        spans.push(Span::styled(&app.owner,   Style::default().fg(DIM)));
        spans.push(Span::styled("/",          Style::default().fg(VDIM)));
        spans.push(Span::styled(&app.repo,    Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled("  ⎇ ",       Style::default().fg(DIM)));
        spans.push(Span::styled(&app.branch,  Style::default().fg(AMBER)));

        if let Some(ref info) = app.repo_info {
            spans.push(Span::styled("  ★ ",    Style::default().fg(DIM)));
            spans.push(Span::styled(
                info.stargazers_count.to_string(),
                Style::default().fg(YELLOW),
            ));
            if let Some(ref lang) = info.language {
                spans.push(Span::styled(format!("  {lang}"), Style::default().fg(DIM)));
            }
            if info.private {
                spans.push(Span::styled(
                    "  PRIVATE",
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                ));
            }
        }
    }

    // Right side: loading indicator + hints
    if app.loading {
        spans.push(Span::styled("  ◌ loading…", Style::default().fg(AMBER2)));
    }
    if !app.selected.is_empty() {
        let n = app.selected.len();
        spans.push(Span::styled(
            format!("  ● {n} selected"),
            Style::default().fg(BLUE),
        ));
    }

    f.render_widget(
        Paragraph::new(Line::from(spans))
            .style(Style::default().bg(BG2)),
        area,
    );
}

fn draw_breadcrumb(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![
        Span::styled(" ~ ", Style::default().fg(AMBER)),
    ];
    if !app.current_path.is_empty() {
        for (i, part) in app.current_path.split('/').enumerate() {
            if i > 0 { spans.push(Span::styled("/", Style::default().fg(VDIM))); }
            spans.push(Span::styled(part, Style::default().fg(MID)));
        }
    }
    // Keybind mini-hints on the right
    let hint_text = "  ?=help  b=branch  /=search  q=quit";
    let hint = Span::styled(hint_text, Style::default().fg(VDIM));

    f.render_widget(
        Paragraph::new(Line::from(spans))
            .style(Style::default().bg(BG2)),
        area,
    );

    // Render hints right-aligned
    let hint_w = hint_text.len() as u16;
    if hint_w < area.width {
        f.render_widget(
            Paragraph::new(Line::from(hint)),
            Rect {
                x:     area.x + area.width - hint_w,
                y:     area.y,
                width: hint_w,
                height: 1,
            },
        );
    }
}

fn draw_content(f: &mut Frame, app: &mut App, area: Rect) {
    if app.preview.is_some() {
        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(area);
        draw_file_list(f, app, h[0]);
        draw_preview(f, app, h[1]);
    } else {
        draw_file_list(f, app, area);
    }
}

fn draw_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    let dirs  = app.files.iter().filter(|f| f.kind == "dir").count();
    let files = app.files.len() - dirs;
    let sel   = app.selected.len();

    let title = if app.search_active || !app.search_query.is_empty() {
        Line::from(vec![
            Span::styled(" / ", Style::default().fg(AMBER)),
            Span::styled(&app.search_query, Style::default().fg(BRIGHT)),
            Span::styled(
                format!("  ({} matches)", app.filtered.len()),
                Style::default().fg(DIM),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!("  {dirs}▸ {files}≡", ),
                Style::default().fg(DIM),
            ),
            if sel > 0 {
                Span::styled(format!("  ● {sel}"), Style::default().fg(BLUE))
            } else {
                Span::raw("")
            },
        ])
    };

    let block = Block::default()
        .borders(Borders::NONE)
        .title(title)
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Build list items
    let name_w = inner.width.saturating_sub(8) as usize; // leave space for icon + size

    let items: Vec<ListItem<'static>> = app.filtered.iter().map(|&fi| {
        let file = &app.files[fi];
        let is_selected = app.selected.contains(&file.path);
        let is_preview  = app.preview_path.as_deref() == Some(file.path.as_str());
        let (icon, icon_col) = file_icon(&file.name, file.kind == "dir");

        // Truncate name
        let name = if file.name.len() > name_w {
            format!("{}…", &file.name[..name_w.saturating_sub(1)])
        } else {
            file.name.clone()
        };

        let size_str: String = if file.kind == "file" {
            file.size.map(fmt_size).unwrap_or_default()
        } else {
            String::new()
        };

        // Left: sel indicator + icon + name
        let sel_span = if is_selected {
            Span::styled("● ", Style::default().fg(BLUE))
        } else if is_preview {
            Span::styled("▶ ", Style::default().fg(AMBER))
        } else {
            Span::styled("  ", Style::default().fg(VDIM))
        };

        let trailing = if file.kind == "dir" { "/" } else { "" };
        let name_col = if file.kind == "dir" { BRIGHT } else { MID };

        // Pad between name and size
        let total_left = 2 + 2 + name.len() + trailing.len(); // sel + icon + name + trailing
        let size_w     = size_str.len();
        let gap        = (inner.width as usize)
            .saturating_sub(total_left + size_w);
        let padding    = " ".repeat(gap.max(1));

        let spans = vec![
            sel_span,
            Span::styled(format!("{icon} "), Style::default().fg(icon_col)),
            Span::styled(format!("{name}{trailing}"), Style::default().fg(name_col)),
            Span::styled(padding,  Style::default().fg(VDIM)),
            Span::styled(size_str, Style::default().fg(VDIM)),
        ];

        ListItem::new(Line::from(spans))
    }).collect();

    let list = List::new(items)
        .highlight_style(
            Style::default().bg(BG3).fg(BRIGHT).add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(""); // we handle our own indicator above

    f.render_stateful_widget(list, inner, &mut app.list_state);

    // Scrollbar
    if app.files.len() > inner.height as usize {
        let scroll_pos = app.list_state.selected().unwrap_or(0);
        let mut sb_state = ScrollbarState::new(app.filtered.len())
            .position(scroll_pos);
        let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(VDIM));
        f.render_stateful_widget(sb, inner, &mut sb_state);
    }
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let file_name = app.preview_path.as_deref()
        .and_then(|p| p.rsplit('/').next())
        .unwrap_or("preview");

    let (icon, icon_col) = file_icon(file_name, false);

    let title = Line::from(vec![
        Span::styled(format!(" {icon} "), Style::default().fg(icon_col)),
        Span::styled(file_name, Style::default().fg(BRIGHT)),
        Span::styled("  Ctrl+j/k scroll  p close ", Style::default().fg(VDIM)),
    ]);

    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(VDIM))
        .title(title)
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let content = app.preview.as_deref().unwrap_or("loading…");

    // Show with line numbers
    let lines_with_nums: Vec<Line<'static>> = content
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let num = format!("{:>4} ", i + 1);
            let text = line.to_string();
            Line::from(vec![
                Span::styled(num,  Style::default().fg(VDIM)),
                Span::styled(text, Style::default().fg(MID)),
            ])
        })
        .collect();

    let total_lines = lines_with_nums.len();
    let para = Paragraph::new(Text::from(lines_with_nums))
        .scroll((app.preview_scroll, 0));
    f.render_widget(para, inner);

    // Scrollbar for preview
    if total_lines > inner.height as usize {
        let mut sb_state = ScrollbarState::new(total_lines)
            .position(app.preview_scroll as usize);
        let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(VDIM));
        f.render_stateful_widget(sb, inner, &mut sb_state);
    }

    // Scroll position indicator (bottom-right of preview)
    if total_lines > 0 {
        let pct = (app.preview_scroll as usize * 100) / total_lines;
        let indicator = format!(" {pct}% ");
        let iw = indicator.len() as u16;
        let ix = area.x + area.width.saturating_sub(iw + 1);
        let iy = area.y + area.height.saturating_sub(1);
        f.render_widget(
            Paragraph::new(Span::styled(indicator, Style::default().fg(VDIM))),
            Rect { x: ix, y: iy, width: iw, height: 1 },
        );
    }
}

fn draw_bottom_bar(f: &mut Frame, app: &App, area: Rect) {
    let (line, bg) = if app.search_active {
        (
            Line::from(vec![
                Span::styled("  / ", Style::default().fg(AMBER)),
                Span::styled(&app.search_query, Style::default().fg(BRIGHT)),
                Span::styled("█", Style::default().fg(AMBER)),
                Span::styled("  Esc=cancel  Enter=confirm", Style::default().fg(DIM)),
            ]),
            BG2,
        )
    } else if let Some(ref err) = app.error {
        (
            Line::from(vec![
                Span::styled("  ✗ ", Style::default().fg(RED)),
                Span::styled(err.as_str(), Style::default().fg(RED)),
                Span::styled("  e=dismiss", Style::default().fg(DIM)),
            ]),
            Color::Rgb(20, 8, 8),
        )
    } else {
        // Show downloads if any active
        let active_dl: Vec<&str> = app.downloads.iter()
            .filter(|d| !d.done && d.error.is_none())
            .map(|d| d.name.as_str())
            .collect();

        if !active_dl.is_empty() {
            (
                Line::from(vec![
                    Span::styled("  ◌ ", Style::default().fg(AMBER)),
                    Span::styled(
                        format!("downloading: {}", active_dl.join(", ")),
                        Style::default().fg(MID),
                    ),
                ]),
                BG2,
            )
        } else {
            (
                Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(&app.status, Style::default().fg(DIM)),
                    Span::styled(
                        "  j/k=nav  Space=sel  d=dl  b=branch  /=search  ?=help",
                        Style::default().fg(VDIM),
                    ),
                ]),
                BG2,
            )
        }
    };

    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(bg)),
        area,
    );
}

// ─── Branch popup ─────────────────────────────────────────────────────────────

fn draw_branch_popup(f: &mut Frame, app: &mut App, area: Rect) {
    let w = 40u16.min(area.width.saturating_sub(4));
    let h = (app.branches.len() as u16 + 4).min(area.height.saturating_sub(4)).max(6);
    let x = (area.width - w) / 2;
    let y = (area.height - h) / 2;
    let popup_area = Rect { x, y, width: w, height: h };

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ⎇ ", Style::default().fg(AMBER)),
            Span::styled("switch branch", Style::default().fg(MID)),
            Span::styled(" ", Style::default()),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(AMBER))
        .style(Style::default().bg(BG2));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let items: Vec<ListItem<'static>> = app.branches.iter().map(|b| {
        let is_current = b == &app.branch;
        let sym = if is_current { "● " } else { "  " };
        let col = if is_current { AMBER } else { MID };
        ListItem::new(Line::from(vec![
            Span::styled(sym, Style::default().fg(col)),
            Span::styled(b.clone(), Style::default().fg(col)),
        ]))
    }).collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(BG3).add_modifier(Modifier::BOLD));

    f.render_stateful_widget(list, inner, &mut app.branch_list_state);

    // Hint at bottom
    let hint_y = popup_area.y + popup_area.height - 1;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            key_span(" Enter"),
            hint_span("=switch  "),
            key_span("Esc"),
            hint_span("=close "),
        ])).alignment(Alignment::Center),
        Rect { x: popup_area.x, y: hint_y, width: popup_area.width, height: 1 },
    );
}

// ─── Help popup ───────────────────────────────────────────────────────────────

fn draw_help_popup(f: &mut Frame, area: Rect) {
    let w = 56u16.min(area.width.saturating_sub(4));
    let h = 26u16.min(area.height.saturating_sub(2));
    let x = (area.width - w) / 2;
    let y = (area.height - h) / 2;
    let popup_area = Rect { x, y, width: w, height: h };

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ? ", Style::default().fg(AMBER)),
            Span::styled("keyboard shortcuts", Style::default().fg(MID)),
            Span::styled(" ", Style::default()),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(AMBER))
        .style(Style::default().bg(BG2));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let bindings: &[(&str, &str, &str)] = &[
        // (section, key, desc)
        ("NAVIGATE", "",          ""),
        ("",         "j / k / ↑↓", "move up / down"),
        ("",         "Enter / l",  "enter dir  or  preview file"),
        ("",         "h / Backspace", "go back"),
        ("",         "g / G",      "jump to top / bottom"),
        ("",         "Ctrl+d/u",   "page down / up"),
        ("",         "",           ""),
        ("SELECT",   "",           ""),
        ("",         "Space",      "toggle select current"),
        ("",         "a",          "select all visible"),
        ("",         "u",          "unselect all"),
        ("",         "",           ""),
        ("ACTIONS",  "",           ""),
        ("",         "d",          "download selected  (or current)"),
        ("",         "D",          "download current file"),
        ("",         "p",          "toggle preview pane"),
        ("",         "c",          "copy raw URL to clipboard"),
        ("",         "w",          "copy wget command"),
        ("",         "/",          "filter files in current dir"),
        ("",         "r",          "refresh current directory"),
        ("",         "",           ""),
        ("UI",       "",           ""),
        ("",         "b",          "switch branch"),
        ("",         "C",          "open config"),
        ("",         "?",          "this help"),
        ("",         "q / Esc",    "back / quit"),
        ("",         "Ctrl+C",     "force quit"),
    ];

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (section, key, desc) in bindings {
        if !section.is_empty() {
            lines.push(Line::from(Span::styled(
                format!(" {section}"),
                Style::default().fg(AMBER2).add_modifier(Modifier::BOLD),
            )));
        } else if key.is_empty() {
            lines.push(Line::from(""));
        } else {
            let padded_key = format!("{:<20}", key);
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(padded_key, Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
                Span::styled(desc.to_string(), Style::default().fg(MID)),
            ]));
        }
    }

    f.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );

    // Dismiss hint
    let hint_y = popup_area.y + popup_area.height - 1;
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " any key to close ",
            Style::default().fg(DIM),
        ))).alignment(Alignment::Center),
        Rect { x: popup_area.x, y: hint_y, width: popup_area.width, height: 1 },
    );
}

// ─── Config screen ────────────────────────────────────────────────────────────

fn draw_config(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let w = 62u16.min(area.width.saturating_sub(4));
    let h = 14u16;
    let x = (area.width - w) / 2;
    let y = area.height.saturating_sub(h) / 2;
    let config_area = Rect { x, y, width: w, height: h };

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ⚙ ", Style::default().fg(AMBER)),
            Span::styled("config", Style::default().fg(MID)),
            Span::styled(
                format!("  ~/.config/riftx/config.json"),
                Style::default().fg(VDIM),
            ),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(AMBER))
        .style(Style::default().bg(BG2));

    let inner = block.inner(config_area);
    f.render_widget(block, config_area);

    let fields = [
        ("GitHub Token", app.config.token.as_deref().map(|t| {
            if t.len() > 8 { format!("{}…{}", &t[..4], &t[t.len()-4..]) }
            else { "set".to_string() }
        }).unwrap_or_else(|| "(not set — 60 req/hr)".to_string())),
        ("Download Path", app.config.download_path.as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| ".".to_string())),
    ];

    for (i, (label, value)) in fields.iter().enumerate() {
        let fy = inner.y + (i as u16) * 3;
        let is_active = app.cfg_field == i;

        let border_col = if is_active { AMBER } else { DIM };
        let field_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_col))
            .title(Line::from(Span::styled(
                format!(" {label} "),
                Style::default().fg(if is_active { AMBER } else { DIM }),
            )));
        let field_area = Rect { x: inner.x, y: fy, width: inner.width, height: 3 };
        let field_inner = field_block.inner(field_area);
        f.render_widget(field_block, field_area);

        let display = if is_active && app.cfg_editing {
            format!("{}_", app.cfg_buf)
        } else {
            value.clone()
        };
        let col = if is_active { BRIGHT } else { MID };
        f.render_widget(
            Paragraph::new(Span::styled(display, Style::default().fg(col))),
            field_inner,
        );
    }

    let hint_y = inner.y + inner.height - 1;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            key_span("Enter"), hint_span("=edit  "),
            key_span("Esc"),   hint_span("=back  "),
            key_span("j/k"),   hint_span("=select field  "),
        ])).alignment(Alignment::Center),
        Rect { x: inner.x, y: hint_y, width: inner.width, height: 1 },
    );
}

// ─── Span helpers ─────────────────────────────────────────────────────────────

fn key_span(s: &'static str) -> Span<'static> {
    Span::styled(s, Style::default().fg(AMBER).add_modifier(Modifier::BOLD))
}

fn hint_span(s: &'static str) -> Span<'static> {
    Span::styled(s, Style::default().fg(DIM))
}
