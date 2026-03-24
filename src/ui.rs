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

use crate::app::{App, DlEntry, FilteredItem, Screen, SearchMode, SortMode};
use crate::providers::{fmt_size, NodeKind};
use crate::theme::Theme;

// ─── Spinner frames ───────────────────────────────────────────────────────────
const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn spinner_frame(tick: u64) -> &'static str {
    SPINNER[(tick / 3) as usize % SPINNER.len()]
}

// ─── Logo ─────────────────────────────────────────────────────────────────────
const LOGO: &[&str] = &[
    "██████╗ ██╗███████╗████████╗██╗  ██╗",
    "██╔══██╗██║██╔════╝╚══██╔══╝╚██╗██╔╝",
    "██████╔╝██║█████╗     ██║    ╚███╔╝ ",
    "██╔══██╗██║██╔══╝     ██║    ██╔██╗ ",
    "██║  ██║██║██║        ██║   ██╔╝ ██╗",
    "╚═════╝ ╚═╝╚═╝        ╚═╝   ╚═╝  ╚═╝",
];

// ─── Top-level draw ───────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &mut App) {
    app.advance_tick();
    let area = f.area();
    let t = &app.theme;
    f.render_widget(Block::default().style(Style::default().bg(t.bg)), area);

    match &app.screen.clone() {
        Screen::Home         => draw_home(f, app, area),
        Screen::Browser      => draw_browser(f, app, area),
        Screen::BranchPopup  => { draw_browser(f, app, area); draw_branch_popup(f, app, area); }
        Screen::DownloadPlan => { draw_browser(f, app, area); draw_plan_popup(f, app, area);   }
        Screen::Help         => { draw_browser(f, app, area); draw_help_popup(f, app, area);   }
        Screen::Config       => draw_config(f, app, area),
        Screen::Downloads    => { draw_browser(f, app, area); draw_downloads_popup(f, app, area); }
    }
}

// ─── Home screen ──────────────────────────────────────────────────────────────

fn draw_home(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let logo_h  = LOGO.len() as u16;
    let hist_h  = app.history.len().min(6) as u16;
    let total_h = logo_h + 2 + 3 + 2 + (if hist_h > 0 { hist_h + 3 } else { 0 });
    let top_y   = area.height.saturating_sub(total_h) / 2;

    // Logo
    {
        let logo_lines: Vec<Line<'static>> = LOGO.iter().enumerate().map(|(i, line)| {
            let color = match i {
                0 | 1 => t.accent,
                2 | 3 => t.accent2,
                _     => t.accent3,
            };
            Line::from(Span::styled(*line, Style::default().fg(color).add_modifier(Modifier::BOLD)))
        }).collect();
        f.render_widget(
            Paragraph::new(Text::from(logo_lines)).alignment(Alignment::Center),
            Rect { x: 0, y: top_y, width: area.width, height: logo_h },
        );
    }

    // Version + tagline row
    let tag_y = top_y + logo_h;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("v0.0.7  ", Style::default().fg(t.vdim)),
            Span::styled("explore remote repos", Style::default().fg(t.dim)),
            Span::styled("  ·  no clone needed", Style::default().fg(t.vdim)),
            Span::styled(
                format!("  [{}]", app.theme_name.as_str()),
                Style::default().fg(t.vdim),
            ),
        ])).alignment(Alignment::Center),
        Rect { x: 0, y: tag_y, width: area.width, height: 1 },
    );

    // Input box
    let box_w = 66u16.min(area.width.saturating_sub(4));
    let box_x = (area.width  - box_w) / 2;
    let box_y = tag_y + 2;
    let border_col = if app.input.is_empty() { t.dim } else { t.accent };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_col))
        .title(Line::from(vec![
            Span::styled(" github / gitlab / codeberg / gitea ", Style::default().fg(t.dim)),
        ]));
    let inner = block.inner(Rect { x: box_x, y: box_y, width: box_w, height: 3 });
    f.render_widget(block, Rect { x: box_x, y: box_y, width: box_w, height: 3 });

    let input_line = if app.input.is_empty() {
        Line::from(Span::styled(
            "owner/repo  or  https://github.com/owner/repo",
            Style::default().fg(Color::Rgb(45, 45, 65)),
        ))
    } else {
        Line::from(Span::styled(app.input.clone(), Style::default().fg(t.bright)))
    };
    f.render_widget(Paragraph::new(input_line), inner);
    let cx = inner.x + (app.input_cursor as u16).min(inner.width.saturating_sub(1));
    f.set_cursor_position((cx, inner.y));

    // Autocomplete dropdown
    let autocomplete_h = app.autocomplete_suggestions.len() as u16;
    if autocomplete_h > 0 {
        let drop_h    = autocomplete_h + 2;
        let drop_rect = Rect { x: box_x, y: box_y + 3, width: box_w, height: drop_h };
        if drop_rect.y + drop_rect.height < area.height {
            f.render_widget(Clear, drop_rect);
            let drop_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(t.dim))
                .style(Style::default().bg(t.bg2));
            let inner_drop = drop_block.inner(drop_rect);
            f.render_widget(drop_block, drop_rect);

            for (i, suggestion) in app.autocomplete_suggestions.iter().enumerate() {
                let is_sel   = app.autocomplete_idx == Some(i);
                let split    = app.input.len().min(suggestion.len());
                let bg       = if is_sel { t.accent } else { t.bg2 };
                let fg_match = if is_sel { t.bg } else { t.accent };
                let fg_rest  = if is_sel { t.bg } else { t.dim };

                let mut row_spans = vec![Span::styled(" ", Style::default())];
                row_spans.push(Span::styled(
                    suggestion[..split].to_string(),
                    Style::default().fg(fg_match).add_modifier(Modifier::BOLD),
                ));
                row_spans.push(Span::styled(
                    suggestion[split..].to_string(),
                    Style::default().fg(fg_rest),
                ));
                if is_sel {
                    row_spans.push(Span::styled("  Tab", Style::default().fg(t.bg)));
                }

                f.render_widget(
                    Paragraph::new(Line::from(row_spans))
                        .style(Style::default().bg(bg)),
                    Rect { x: inner_drop.x, y: inner_drop.y + i as u16,
                           width: inner_drop.width, height: 1 },
                );
            }
        }
    }

    // Hints
    let hint_y = box_y + 3 + if autocomplete_h > 0 { autocomplete_h + 2 } else { 0 };
    let hint_line = if app.input.is_empty() {
        Line::from(vec![
            key_span("Enter", t.accent), hint_span(" load  ", t.dim),
            key_span("↑",     t.accent), hint_span(" history  ", t.dim),
            key_span("1-6",   t.accent2), hint_span(" recent  ", t.dim),
            key_span("T",     t.accent), hint_span(" theme  ", t.dim),
            key_span("C",     t.accent), hint_span(" config  ", t.dim),
            key_span("q",     t.accent), hint_span(" quit", t.dim),
        ])
    } else {
        Line::from(vec![
            key_span("Enter",  t.accent),  hint_span(" load  ", t.dim),
            key_span("Tab",    t.accent2), hint_span(" complete  ", t.dim),
            key_span("↑↓",    t.accent2), hint_span(" suggestions  ", t.dim),
            key_span("Esc",    t.dim),     hint_span(" clear", t.dim),
        ])
    };
    if hint_y < area.height {
        f.render_widget(
            Paragraph::new(hint_line).alignment(Alignment::Center),
            Rect { x: 0, y: hint_y, width: area.width, height: 1 },
        );
    }

    // Recent history section
    if !app.history.is_empty() {
        let hist_y = hint_y + 2;
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  RECENT  ", Style::default().fg(t.accent2).add_modifier(Modifier::BOLD)),
                Span::styled("press 1-6 to load", Style::default().fg(Color::Rgb(55,55,75))),
            ])).alignment(Alignment::Center),
            Rect { x: 0, y: hist_y, width: area.width, height: 1 },
        );
        for (i, h) in app.history.iter().enumerate().take(6) {
            let y = hist_y + 1 + i as u16;
            if y >= area.height { break; }
            let prov_col = provider_color(&h.provider, t);
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(format!(" {} ", i + 1), Style::default().fg(t.vdim)),
                    Span::styled(format!("[{}] ", h.provider.to_uppercase()),
                        Style::default().fg(prov_col).add_modifier(Modifier::BOLD)),
                    Span::styled(&h.owner,  Style::default().fg(t.accent2)),
                    Span::styled("/",        Style::default().fg(t.vdim)),
                    Span::styled(&h.repo,   Style::default().fg(t.bright).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("  ⎇ {}", h.branch), Style::default().fg(t.vdim)),
                ])).alignment(Alignment::Center),
                Rect { x: 0, y, width: area.width, height: 1 },
            );
        }
    }

    // Error display
    if let Some(ref err) = app.error {
        let y = area.height.saturating_sub(2);
        f.render_widget(
            Paragraph::new(Span::styled(format!("  ✗  {err}"), Style::default().fg(t.red))),
            Rect { x: 0, y, width: area.width, height: 1 },
        );
    }

    // Animated spinner in top-right
    if app.loading {
        let frame = spinner_frame(app.tick);
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(" {frame} "),
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            )),
            Rect { x: area.width.saturating_sub(4), y: 0, width: 4, height: 1 },
        );
    }
}

// ─── Browser screen ───────────────────────────────────────────────────────────

fn draw_browser(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // top bar
            Constraint::Length(1),  // breadcrumb
            Constraint::Min(1),     // content
            Constraint::Length(1),  // bottom bar
        ])
        .split(area);

    draw_top_bar(f, app, chunks[0]);
    draw_breadcrumb(f, app, chunks[1]);
    draw_content(f, app, chunks[2]);
    draw_bottom_bar(f, app, chunks[3]);
}

fn draw_top_bar(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let badge = app.provider.badge();
    let badge_col = provider_color(&app.provider.label().to_lowercase(), t);

    let mut spans = vec![
        Span::raw(" "),
        Span::styled("rift", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
        Span::styled("x",    Style::default().fg(t.orange).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  [{badge}]"), Style::default().fg(badge_col).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
    ];

    if !app.owner.is_empty() {
        spans.push(Span::styled(&app.owner, Style::default().fg(t.dim)));
        spans.push(Span::styled("/",         Style::default().fg(t.vdim)));
        spans.push(Span::styled(&app.repo,  Style::default().fg(t.bright).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled("  ⎇ ",      Style::default().fg(t.dim)));
        spans.push(Span::styled(&app.branch, Style::default().fg(t.accent)));

        if let Some(ref meta) = app.repo_meta {
            spans.push(Span::styled("  ★ ", Style::default().fg(t.dim)));
            spans.push(Span::styled(meta.stars.to_string(), Style::default().fg(t.yellow)));
            if let Some(ref lang) = meta.language {
                spans.push(Span::styled(format!("  {lang}"), Style::default().fg(t.dim)));
            }
            if meta.private {
                spans.push(Span::styled("  🔒",
                    Style::default().fg(t.accent).add_modifier(Modifier::BOLD)));
            }
        }
    }

    // Sort mode badge (only show if non-default)
    if app.sort_mode != SortMode::Default {
        spans.push(Span::styled(
            format!("  ⇅{}", app.sort_mode.label()),
            Style::default().fg(t.cyan).add_modifier(Modifier::BOLD),
        ));
    }

    // Size filter badge
    if let Some(min) = app.min_size {
        spans.push(Span::styled(
            format!("  >{}", fmt_size(min)),
            Style::default().fg(t.teal).add_modifier(Modifier::BOLD),
        ));
    }

    // Extension filter badge
    if let Some(ref ext) = app.ext_filter {
        spans.push(Span::styled("  .", Style::default().fg(t.dim)));
        spans.push(Span::styled(ext.clone(), Style::default().fg(t.teal).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled("  x=clear", Style::default().fg(t.vdim)));
    }

    if !app.search_query.is_empty() {
        spans.push(Span::styled(
            format!("  {} match{}", app.filtered.len(),
                if app.filtered.len() == 1 { "" } else { "es" }),
            Style::default().fg(t.accent2),
        ));
    }

    // Bookmarks count
    if !app.bookmarks.is_empty() {
        spans.push(Span::styled(
            format!("  ★{}", app.bookmarks.len()),
            Style::default().fg(t.yellow),
        ));
    }

    // Active download indicator
    let active_dl = app.downloads.iter().filter(|d| !d.done && d.error.is_none()).count();
    let done_dl   = app.downloads.iter().filter(|d| d.done).count();
    if active_dl > 0 {
        let frame = spinner_frame(app.tick);
        spans.push(Span::styled(
            format!("  {frame} {active_dl}↓"),
            Style::default().fg(t.green).add_modifier(Modifier::BOLD),
        ));
    } else if done_dl > 0 {
        spans.push(Span::styled(
            format!("  ✓ {done_dl}"),
            Style::default().fg(t.teal),
        ));
    }

    if app.loading {
        let frame = spinner_frame(app.tick);
        spans.push(Span::styled(
            format!("  {frame} loading…"),
            Style::default().fg(t.accent2),
        ));
    }
    if !app.selected.is_empty() {
        spans.push(Span::styled(
            format!("  ● {} sel", app.selected.len()),
            Style::default().fg(t.blue).add_modifier(Modifier::BOLD),
        ));
    }

    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(t.bg2)),
        area,
    );
}

fn draw_breadcrumb(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let mut spans = vec![Span::styled(" ~ ", Style::default().fg(t.accent))];
    if !app.current_path.is_empty() {
        for (i, part) in app.current_path.split('/').enumerate() {
            if i > 0 { spans.push(Span::styled("/", Style::default().fg(t.vdim))); }
            spans.push(Span::styled(part, Style::default().fg(t.mid)));
        }
    }

    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(t.bg2)),
        area,
    );
    let hint = "  ?=help  b=branch  /=search  S=sort  f=size  m=pin  n=new  T=theme  q=quit";
    let hint_w = hint.len() as u16;
    if hint_w < area.width {
        f.render_widget(
            Paragraph::new(Span::styled(hint, Style::default().fg(Color::Rgb(55,55,75)))),
            Rect { x: area.x + area.width - hint_w, y: area.y, width: hint_w, height: 1 },
        );
    }
}

fn draw_content(f: &mut Frame, app: &mut App, area: Rect) {
    if app.preview.is_some() {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(area);
        draw_file_list(f, app, chunks[0]);
        draw_preview(f, app, chunks[1]);
    } else {
        draw_file_list(f, app, area);
    }
}

fn draw_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    let t   = &app.theme;
    let sel = app.selected.len();

    let title = if app.search_mode != SearchMode::Off || !app.search_query.is_empty() {
        let sym = match app.search_mode {
            SearchMode::Name => "/ ",
            SearchMode::Ext  => "%.ext ",
            SearchMode::Path => "\\ ",
            SearchMode::Off  => "/ ",
        };
        let cursor = if app.search_mode != SearchMode::Off { "█" } else { "" };
        Line::from(vec![
            Span::styled(sym, Style::default().fg(t.accent)),
            Span::styled(&app.search_query, Style::default().fg(t.bright)),
            Span::styled(cursor, Style::default().fg(t.accent)),
            Span::styled(
                format!("  {} match{}", app.filtered.len(),
                    if app.filtered.len() == 1 { "" } else { "es" }),
                Style::default().fg(t.dim),
            ),
        ])
    } else {
        let dirs  = app.files.iter().filter(|f| f.kind == NodeKind::Dir).count();
        let files = app.files.len() - dirs;
        let mut title_spans = vec![
            Span::styled(format!("  {dirs}▸ {files}≡"), Style::default().fg(t.dim)),
        ];
        if sel > 0 {
            title_spans.push(Span::styled(format!("  ● {sel}"), Style::default().fg(t.blue)));
        }
        if app.sort_mode != SortMode::Default {
            title_spans.push(Span::styled(
                format!("  ⇅{}", app.sort_mode.label()),
                Style::default().fg(t.cyan),
            ));
        }
        Line::from(title_spans)
    };

    let block = Block::default().borders(Borders::NONE)
        .title(title).style(Style::default().bg(t.bg));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let name_w = inner.width.saturating_sub(10) as usize;

    let items: Vec<ListItem<'static>> = app.filtered.iter().map(|fi: &FilteredItem| {
        let node       = &app.files[fi.idx];
        let is_sel     = app.selected.contains(&node.path);
        let is_preview = app.preview_path.as_deref() == Some(node.path.as_str());
        let is_bmark   = app.is_bookmarked(&node.path);
        let (icon, icon_col) = file_icon(&node.name, node.kind == NodeKind::Dir, t);
        let is_dir = node.kind == NodeKind::Dir;

        let name_raw = if node.name.len() > name_w {
            format!("{}…", &node.name[..name_w.saturating_sub(1)])
        } else { node.name.clone() };

        let name_col = if is_dir { t.bright } else { t.mid };
        let name_spans = fuzzy_highlight_spans(
            &name_raw,
            fi.fuzzy.as_ref().map(|fm| fm.positions.as_slice()).unwrap_or(&[]),
            name_col,
            t.match_hl,
        );

        // Selection / preview / bookmark indicator (2 chars)
        let sel_span = if is_sel {
            Span::styled("● ", Style::default().fg(t.blue))
        } else if is_preview {
            Span::styled("▶ ", Style::default().fg(t.accent))
        } else if is_bmark {
            Span::styled("★ ", Style::default().fg(t.yellow))
        } else {
            Span::styled("  ", Style::default())
        };

        let trailing  = if is_dir { "/" } else { "" };
        let size_str  = if !is_dir { node.size.map(fmt_size).unwrap_or_default() } else { String::new() };
        let total_left = 2 + 2 + name_raw.len() + trailing.len();
        let gap = (inner.width as usize).saturating_sub(total_left + size_str.len());

        let mut spans = vec![sel_span, Span::styled(format!("{icon} "), Style::default().fg(icon_col))];
        spans.extend(name_spans);
        if is_dir { spans.push(Span::styled("/", Style::default().fg(t.dim))); }
        spans.push(Span::styled(" ".repeat(gap.max(1)), Style::default()));
        if !size_str.is_empty() {
            // Highlight size if above the current min_size filter
            let size_col = if let Some(min) = app.min_size {
                if node.size.unwrap_or(0) >= min { t.teal } else { t.vdim }
            } else { t.vdim };
            spans.push(Span::styled(size_str, Style::default().fg(size_col)));
        }

        ListItem::new(Line::from(spans))
    }).collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(t.bg3).fg(t.bright).add_modifier(Modifier::BOLD))
        .highlight_symbol("");

    f.render_stateful_widget(list, inner, &mut app.list_state);

    if app.files.len() > inner.height as usize {
        let pos = app.list_state.selected().unwrap_or(0);
        let mut sb = ScrollbarState::new(app.filtered.len()).position(pos);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight).style(Style::default().fg(t.vdim)),
            inner, &mut sb,
        );
    }
}

fn fuzzy_highlight_spans<'a>(
    text:      &str,
    positions: &[usize],
    base_col:  Color,
    hl_col:    Color,
) -> Vec<Span<'static>> {
    if positions.is_empty() {
        return vec![Span::styled(text.to_string(), Style::default().fg(base_col))];
    }
    let chars: Vec<char> = text.chars().collect();
    let mut spans = Vec::new();
    let mut buf   = String::new();
    let mut in_hl = false;

    for (i, &ch) in chars.iter().enumerate() {
        let should_hl = positions.contains(&i);
        if should_hl != in_hl {
            if !buf.is_empty() {
                let col = if in_hl { hl_col } else { base_col };
                let st  = if in_hl { Style::default().fg(col).add_modifier(Modifier::BOLD) }
                           else     { Style::default().fg(col) };
                spans.push(Span::styled(buf.clone(), st));
                buf.clear();
            }
            in_hl = should_hl;
        }
        buf.push(ch);
    }
    if !buf.is_empty() {
        let col = if in_hl { hl_col } else { base_col };
        let st  = if in_hl { Style::default().fg(col).add_modifier(Modifier::BOLD) }
                   else     { Style::default().fg(col) };
        spans.push(Span::styled(buf, st));
    }
    spans
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let file_name = app.preview_path.as_deref()
        .and_then(|p| p.rsplit('/').next()).unwrap_or("preview");
    let (icon, icon_col) = file_icon(file_name, false, t);

    let total_lines = app.preview.as_deref().unwrap_or("").lines().count();
    let lines_label = if total_lines > 0 { format!("  {total_lines}L") } else { String::new() };
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(t.accent2))
        .title(Line::from(vec![
            Span::styled(format!(" {icon} "), Style::default().fg(icon_col)),
            Span::styled(file_name, Style::default().fg(t.bright).add_modifier(Modifier::BOLD)),
            Span::styled(lines_label, Style::default().fg(t.vdim)),
            Span::styled("  Ctrl+j/k  p=close", Style::default().fg(t.vdim)),
        ]))
        .style(Style::default().bg(t.bg));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let content = app.preview.as_deref().unwrap_or("loading…");
    let lines: Vec<Line<'static>> = content.lines().enumerate().map(|(i, l)| {
        let row_bg = if i % 2 == 0 { t.bg } else { t.bg2 };
        Line::from(vec![
            Span::styled(format!("{:>4} ", i + 1), Style::default().fg(t.vdim)),
            Span::styled(l.to_string(), Style::default().fg(t.mid)),
        ]).style(Style::default().bg(row_bg))
    }).collect();
    let total = lines.len();
    f.render_widget(Paragraph::new(Text::from(lines)).scroll((app.preview_scroll, 0)), inner);

    if total > inner.height as usize {
        let mut sb = ScrollbarState::new(total).position(app.preview_scroll as usize);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight).style(Style::default().fg(t.vdim)),
            inner, &mut sb,
        );
        let pct = (app.preview_scroll as usize * 100) / total.max(1);
        let ind = format!(" {pct}% ");
        let iw  = ind.len() as u16;
        f.render_widget(
            Paragraph::new(Span::styled(ind, Style::default().fg(t.vdim))),
            Rect { x: area.x + area.width.saturating_sub(iw + 1),
                   y: area.y + area.height.saturating_sub(1), width: iw, height: 1 },
        );
    }
}

fn draw_bottom_bar(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let (line, bg) = if app.search_mode != SearchMode::Off {
        let sym = match app.search_mode {
            SearchMode::Name => "/ ",
            SearchMode::Ext  => "%.ext ",
            SearchMode::Path => "\\ path ",
            SearchMode::Off  => "/ ",
        };
        (Line::from(vec![
            Span::styled(format!("  {sym}"), Style::default().fg(t.accent)),
            Span::styled(&app.search_query, Style::default().fg(t.bright)),
            Span::styled("█", Style::default().fg(t.accent)),
            Span::styled("  Esc=cancel  Enter=confirm", Style::default().fg(t.dim)),
        ]), t.bg2)
    } else if let Some(ref err) = app.error {
        (Line::from(vec![
            Span::styled("  ✗ ", Style::default().fg(t.red)),
            Span::styled(err.as_str(), Style::default().fg(t.red)),
            Span::styled("  e=dismiss", Style::default().fg(t.dim)),
        ]), Color::Rgb(20, 8, 8))
    } else {
        let active: Vec<&DlEntry> = app.downloads.iter()
            .filter(|d| !d.done && d.error.is_none()).collect();
        let done_count = app.downloads.iter().filter(|d| d.done).count();
        let err_count  = app.downloads.iter().filter(|d| d.error.is_some()).count();

        if !active.is_empty() {
            let names: Vec<&str> = active.iter().map(|d| d.name.as_str()).take(2).collect();
            let more = if active.len() > 2 { format!(" +{}", active.len()-2) } else { String::new() };
            (Line::from(vec![
                Span::styled("  ↓ ", Style::default().fg(t.green)),
                Span::styled(names.join(", "), Style::default().fg(t.mid)),
                Span::styled(more, Style::default().fg(t.dim)),
                if done_count > 0 {
                    Span::styled(format!("   ✓ {done_count}"), Style::default().fg(t.vdim))
                } else { Span::raw("") },
                if err_count > 0 {
                    Span::styled(format!("  ✗ {err_count}"), Style::default().fg(t.red))
                } else { Span::raw("") },
                Span::styled("  O=view", Style::default().fg(t.vdim)),
            ]), t.bg2)
        } else {
            let sel = app.selected.len();
            let left = if sel > 0 {
                Span::styled(format!("  ● {sel} selected  Space=toggle  d=plan  D=now  u=clear  "), Style::default().fg(t.blue))
            } else {
                Span::styled(app.status.clone(), Style::default().fg(t.dim))
            };
            (Line::from(vec![
                left,
                Span::styled(
                    " j/k=nav  Enter=open  Space=sel  /=search  d=plan  p=preview  S=sort  f=size  m=pin  n=new  ?=help",
                    Style::default().fg(t.vdim),
                ),
            ]), t.bg2)
        }
    };
    f.render_widget(Paragraph::new(line).style(Style::default().bg(bg)), area);
}

// ─── Downloads popup ──────────────────────────────────────────────────────────

fn draw_downloads_popup(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let popup_w = 72u16.min(area.width.saturating_sub(4));
    let max_h   = (app.downloads.len() as u16 + 6).min(area.height.saturating_sub(4)).max(8);
    let popup   = centered_rect(popup_w, max_h, area);

    f.render_widget(Clear, popup);
    let active = app.downloads.iter().filter(|d| !d.done && d.error.is_none()).count();
    let done   = app.downloads.iter().filter(|d| d.done).count();
    let failed = app.downloads.iter().filter(|d| d.error.is_some()).count();

    let title_col = if active > 0 { t.green } else if failed > 0 { t.red } else { t.accent };
    let spinner   = if active > 0 { format!("{} ", spinner_frame(app.tick)) } else { "↓ ".into() };
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(format!(" {spinner}"), Style::default().fg(title_col)),
            Span::styled("downloads", Style::default().fg(t.bright)),
            Span::styled(
                format!("  {active}↓  {done}✓  {failed}✗"),
                Style::default().fg(t.vdim),
            ),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(title_col))
        .style(Style::default().bg(t.bg2));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let list_h = inner.height.saturating_sub(2);
    for (i, dl) in app.downloads.iter().enumerate().take(list_h as usize) {
        let y = inner.y + i as u16;
        if y >= inner.y + list_h { break; }

        let (status_icon, status_col) = if dl.error.is_some() {
            ("✗", t.red)
        } else if dl.skipped {
            ("~", t.vdim)
        } else if dl.done {
            ("✓", t.green)
        } else {
            (spinner_frame(app.tick), t.accent)
        };

        let (icon, ic) = file_icon(&dl.name, false, t);
        let name_w = inner.width.saturating_sub(6) as usize;
        let name = if dl.name.len() > name_w {
            format!("{}…", &dl.name[..name_w.saturating_sub(1)])
        } else { dl.name.clone() };

        let detail = if let Some(ref e) = dl.error {
            Span::styled(format!("  {}", &e[..e.len().min(30)]), Style::default().fg(t.red))
        } else if dl.skipped {
            Span::styled("  skipped", Style::default().fg(t.vdim))
        } else if dl.done {
            Span::styled("  saved", Style::default().fg(t.vdim))
        } else {
            Span::styled("  …", Style::default().fg(t.dim))
        };

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!(" {status_icon} "), Style::default().fg(status_col)),
                Span::styled(format!("{icon} "), Style::default().fg(ic)),
                Span::styled(name, Style::default().fg(t.mid)),
                detail,
            ])),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
    }

    let hy = inner.y + inner.height - 1;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            key_span(" c", t.dim), hint_span("=clear done  ", t.dim),
            key_span("Esc", t.dim), hint_span("=close", t.dim),
        ])).alignment(Alignment::Center),
        Rect { x: inner.x, y: hy, width: inner.width, height: 1 },
    );
}

// ─── Branch popup ─────────────────────────────────────────────────────────────

fn draw_branch_popup(f: &mut Frame, app: &mut App, area: Rect) {
    let t = &app.theme;
    let w = 44u16.min(area.width.saturating_sub(4));
    let h = (app.branches.len() as u16 + 4).min(area.height.saturating_sub(4)).max(6);
    let popup = centered_rect(w, h, area);

    f.render_widget(Clear, popup);
    let block = popup_block(" ⎇  switch branch ", t);
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let items: Vec<ListItem<'static>> = app.branches.iter().map(|b| {
        let cur = b == &app.branch;
        ListItem::new(Line::from(vec![
            Span::styled(if cur { "● " } else { "  " },
                Style::default().fg(if cur { t.accent } else { t.mid })),
            Span::styled(b.clone(), Style::default().fg(if cur { t.accent } else { t.mid })),
        ]))
    }).collect();

    f.render_stateful_widget(
        List::new(items).highlight_style(Style::default().bg(t.bg3).add_modifier(Modifier::BOLD)),
        inner, &mut app.branch_list_state,
    );

    let hy = popup.y + popup.height - 1;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            key_span(" Enter", t.accent), hint_span("=switch  ", t.dim),
            key_span("Esc",    t.accent), hint_span("=close", t.dim),
        ])).alignment(Alignment::Center),
        Rect { x: popup.x, y: hy, width: popup.width, height: 1 },
    );
}

// ─── Download plan popup ──────────────────────────────────────────────────────

fn draw_plan_popup(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let list_h   = (app.plan.len() as u16).min(14);
    let popup_h  = (list_h + 10).min(area.height.saturating_sub(4));
    let popup_w  = 68u16.min(area.width.saturating_sub(4));
    let popup    = centered_rect(popup_w, popup_h, area);

    f.render_widget(Clear, popup);
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ↓ ", Style::default().fg(t.green)),
            Span::styled("download plan", Style::default().fg(t.bright)),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.green))
        .style(Style::default().bg(t.bg2));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let total_size = app.plan_total_size();
    let count      = app.plan.len();
    let summary    = format!(
        "  {} file{}  ·  total: {}",
        count, if count == 1 { "" } else { "s" },
        if total_size > 0 { fmt_size(total_size) } else { "unknown".into() }
    );
    f.render_widget(
        Paragraph::new(Span::styled(&summary, Style::default().fg(t.mid))),
        Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 },
    );

    let opts_y = inner.y + 1;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("R", Style::default().fg(if app.dl_recursive { t.green } else { t.vdim }).add_modifier(Modifier::BOLD)),
            Span::styled("=recurse  ", Style::default().fg(t.vdim)),
            Span::styled("S", Style::default().fg(if app.dl_preserve_structure { t.green } else { t.vdim }).add_modifier(Modifier::BOLD)),
            Span::styled("=preserve-dirs  ", Style::default().fg(t.vdim)),
            Span::styled("K", Style::default().fg(if app.dl_skip_existing { t.teal } else { t.vdim }).add_modifier(Modifier::BOLD)),
            Span::styled("=skip-existing", Style::default().fg(t.vdim)),
        ])),
        Rect { x: inner.x, y: opts_y, width: inner.width, height: 1 },
    );

    f.render_widget(
        Paragraph::new(Span::styled("─".repeat(inner.width as usize), Style::default().fg(t.vdim))),
        Rect { x: inner.x, y: opts_y + 1, width: inner.width, height: 1 },
    );

    let file_lines: Vec<Line<'static>> = app.plan.iter().map(|pi| {
        let size_s = pi.size.map(fmt_size).unwrap_or_default();
        let (icon, ic) = file_icon(&pi.name, false, t);
        let name_w = inner.width.saturating_sub(4 + size_s.len() as u16) as usize;
        let name   = if pi.name.len() > name_w {
            format!("{}…", &pi.name[..name_w.saturating_sub(1)])
        } else { pi.name.clone() };
        let gap = " ".repeat((inner.width as usize).saturating_sub(2 + 2 + name.len() + size_s.len()).max(1));
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(format!("{icon} "), Style::default().fg(ic)),
            Span::styled(name,  Style::default().fg(t.mid)),
            Span::styled(gap,   Style::default()),
            Span::styled(size_s, Style::default().fg(t.vdim)),
        ])
    }).collect();

    f.render_widget(
        Paragraph::new(Text::from(file_lines)),
        Rect { x: inner.x, y: opts_y + 2, width: inner.width, height: list_h },
    );

    let sep_y = inner.y + inner.height - 2;
    f.render_widget(
        Paragraph::new(Span::styled("─".repeat(inner.width as usize), Style::default().fg(t.vdim))),
        Rect { x: inner.x, y: sep_y, width: inner.width, height: 1 },
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![
            key_span(" Enter", t.green), hint_span("/", t.dim),
            key_span("y",      t.green), hint_span("=go   ", t.dim),
            key_span("R",      t.accent2), hint_span("=recurse  ", t.dim),
            key_span("S",      t.accent2), hint_span("=struct  ", t.dim),
            key_span("Esc",    t.dim),   hint_span("=cancel ", t.dim),
        ])).alignment(Alignment::Center),
        Rect { x: inner.x, y: sep_y + 1, width: inner.width, height: 1 },
    );
}

// ─── Help popup ───────────────────────────────────────────────────────────────

fn draw_help_popup(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let w = 74u16.min(area.width.saturating_sub(4));
    let h = 44u16.min(area.height.saturating_sub(2));
    let popup = centered_rect(w, h, area);

    f.render_widget(Clear, popup);
    let block = popup_block(" ?  keyboard shortcuts ", t);
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let bindings: &[(&str, &str, &str)] = &[
        ("▸  NAVIGATE",  "",              ""),
        ("",             "j / k  ↑ ↓",  "move up / down"),
        ("",             "Enter / l →",  "enter dir  ·  preview file"),
        ("",             "h / Bksp ←",   "go back"),
        ("",             "g / G",         "jump to top / bottom"),
        ("",             "Ctrl+d / u",    "page down / up"),
        ("",             "n",             "new repo (go to home)"),
        ("",             "",              ""),
        ("●  SELECT",    "",              ""),
        ("",             "Space",         "toggle select current"),
        ("",             "a",             "select all visible"),
        ("",             "u",             "unselect all"),
        ("",             "i / I",         "invert selection"),
        ("",             "",              ""),
        ("/  SEARCH",    "",              ""),
        ("",             "/",             "fuzzy search by name"),
        ("",             "%",             "filter by extension"),
        ("",             "\\",            "search by full path"),
        ("",             "x",             "clear all filters"),
        ("",             "",              ""),
        ("⇅  SORT & FILTER", "",         ""),
        ("",             "S",             "cycle sort  (default→name→size↓→ext)"),
        ("",             "f",             "cycle size filter  (off→1K→100K→1M)"),
        ("",             "m",             "toggle bookmark / pin current file"),
        ("",             "",              ""),
        ("↓  DOWNLOAD",  "",              ""),
        ("",             "d",             "open download plan"),
        ("",             "D",             "instant download current file"),
        ("",             "O",             "view downloads panel"),
        ("",             "R  (in plan)",  "toggle recursive folder download"),
        ("",             "S  (in plan)",  "toggle preserve directory structure"),
        ("",             "K  (in plan)",  "toggle skip existing files"),
        ("",             "p",             "toggle inline preview"),
        ("",             "c / w",         "copy raw URL / wget command"),
        ("",             "r",             "refresh current dir"),
        ("",             "",              ""),
        ("⚙  UI",        "",              ""),
        ("",             "b",             "switch branch"),
        ("",             "T / Ctrl+t",    "cycle colour theme"),
        ("",             "C",             "open config"),
        ("",             "?",             "this help"),
        ("",             "q / Esc",       "back / quit"),
    ];

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (section, key, desc) in bindings {
        if !section.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(format!(" {section} "),
                    Style::default().fg(t.accent2).add_modifier(Modifier::BOLD)),
            ]).style(Style::default().bg(t.bg3)));
        } else if key.is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(format!("{:<26}", key),
                    Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
                Span::styled(desc.to_string(), Style::default().fg(t.mid)),
            ]));
        }
    }

    f.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
    f.render_widget(
        Paragraph::new(Span::styled(" any key to close ", Style::default().fg(t.dim)))
            .alignment(Alignment::Center),
        Rect { x: popup.x, y: popup.y + popup.height - 1, width: popup.width, height: 1 },
    );
}

// ─── Config screen ────────────────────────────────────────────────────────────

fn draw_config(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    f.render_widget(Block::default().style(Style::default().bg(t.bg)), area);

    let w = 70u16.min(area.width.saturating_sub(4));
    let h = 30u16.min(area.height.saturating_sub(4));
    let popup = centered_rect(w, h, area);

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ⚙ ", Style::default().fg(t.accent)),
            Span::styled("config", Style::default().fg(t.mid)),
            Span::styled("  ~/.config/riftx/config.toml", Style::default().fg(t.vdim)),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent))
        .style(Style::default().bg(t.bg2));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  AUTH TOKENS", Style::default().fg(t.accent2).add_modifier(Modifier::BOLD)),
        ])).style(Style::default().bg(t.bg3)),
        Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 },
    );

    let fields: &[(&str, &str)] = &[
        ("GitHub Token",   "github_token"),
        ("GitLab Token",   "gitlab_token"),
        ("Codeberg Token", "codeberg_token"),
        ("Gitea Token",    "gitea_token"),
        ("Gitea URL",      "gitea_url (self-hosted)"),
        ("Download Path",  "download_path"),
    ];

    for (idx, (label, hint)) in fields.iter().enumerate() {
        let fy = inner.y + 1 + (idx as u16) * 3;
        if fy + 3 > inner.y + inner.height { break; }
        let active = app.cfg_field == idx;
        let bc     = if active { t.accent } else { t.dim };

        let value = if active && app.cfg_editing {
            format!("{}_", app.cfg_buf)
        } else {
            let raw = app.cfg_field_value_pub(idx);
            if raw.is_empty() { format!("(not set)  # {hint}") }
            else if idx < 4 && raw.len() > 8 {
                format!("{}…{}", &raw[..4], &raw[raw.len()-4..])
            } else { raw }
        };

        let fb = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(bc))
            .title(Line::from(Span::styled(
                format!(" {label} "),
                Style::default().fg(bc),
            )));
        let fa = Rect { x: inner.x, y: fy, width: inner.width, height: 3 };
        let fi = fb.inner(fa);
        f.render_widget(fb, fa);
        f.render_widget(
            Paragraph::new(Span::styled(
                value, Style::default().fg(if active { t.bright } else { t.mid }),
            )),
            fi,
        );
    }

    let dl_y = inner.y + 1 + 6 * 3;
    if dl_y + 2 < inner.y + inner.height {
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  DOWNLOAD DEFAULTS", Style::default().fg(t.accent2).add_modifier(Modifier::BOLD)),
            ])).style(Style::default().bg(t.bg3)),
            Rect { x: inner.x, y: dl_y, width: inner.width, height: 1 },
        );
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  parallel=", Style::default().fg(t.dim)),
                Span::styled(app.config.core.parallel.to_string(), Style::default().fg(t.accent)),
                Span::styled("  retry=", Style::default().fg(t.dim)),
                Span::styled(app.config.core.retry_count.to_string(), Style::default().fg(t.accent)),
                Span::styled("  recursive=", Style::default().fg(t.dim)),
                Span::styled(
                    if app.config.core.recursive { "yes" } else { "no" },
                    Style::default().fg(if app.config.core.recursive { t.green } else { t.dim }),
                ),
            ])),
            Rect { x: inner.x, y: dl_y + 1, width: inner.width, height: 1 },
        );
    }

    let theme_y = inner.y + inner.height - 3;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Theme: ", Style::default().fg(t.dim)),
            Span::styled(app.theme_name.as_str(),
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
            Span::styled("  T / Ctrl+t to cycle", Style::default().fg(t.vdim)),
        ])),
        Rect { x: inner.x, y: theme_y, width: inner.width, height: 1 },
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            key_span(" Enter",  t.accent), hint_span("=edit  ", t.dim),
            key_span("Esc",     t.accent), hint_span("=back  ", t.dim),
            key_span("j/k ↑↓", t.accent), hint_span("=field  ", t.dim),
            key_span("Enter",   t.green),  hint_span("=save when editing", t.dim),
        ])).alignment(Alignment::Center),
        Rect { x: inner.x, y: inner.y + inner.height - 1, width: inner.width, height: 1 },
    );
}

// ─── File icons ───────────────────────────────────────────────────────────────

fn file_icon<'a>(name: &str, is_dir: bool, t: &Theme) -> (&'static str, Color) {
    if is_dir { return ("▸", t.accent); }
    let ext   = name.rsplit('.').next().unwrap_or("").to_lowercase();
    let lower = name.to_lowercase();
    if lower == "dockerfile"                      { return ("■", t.cyan);   }
    if lower.starts_with("makefile")             { return ("■", t.yellow); }
    if lower.starts_with(".git")                 { return ("■", t.dim);    }
    if lower == "license" || lower == "licence"  { return ("≡", t.mid);    }
    if lower.contains("readme")                  { return ("≡", t.blue);   }
    if lower.ends_with(".lock") || ext == "lock" { return ("-", t.dim);    }

    match ext.as_str() {
        "rs"                               => ("●", t.orange),
        "js"|"mjs"|"cjs"                   => ("◆", t.yellow),
        "ts"                               => ("◆", t.blue),
        "tsx"|"jsx"                        => ("◆", t.cyan),
        "py"|"pyw"                         => ("◆", t.green),
        "go"                               => ("◆", t.cyan),
        "rb"                               => ("◆", t.red),
        "java"|"kt"|"kts"                  => ("◆", t.orange),
        "cpp"|"cc"|"cxx"|"c"|"h"|"hpp"    => ("◆", t.purple),
        "swift"                            => ("◆", t.orange),
        "dart"                             => ("◆", t.blue),
        "zig"                              => ("◆", t.accent),
        "ex"|"exs"                         => ("◆", t.purple),
        "hs"|"lhs"                         => ("◆", t.purple),
        "lua"                              => ("◆", t.blue),
        "php"                              => ("◆", t.purple),
        "cs"                               => ("◆", t.purple),
        "scala"|"sbt"                      => ("◆", t.red),
        "nim"                              => ("◆", t.yellow),
        "v"|"vlang"                        => ("◆", t.blue),
        "r"                                => ("◆", t.blue),
        "jl"                               => ("◆", t.purple),
        "md"|"mdx"|"markdown"              => ("≡", t.mid),
        "txt"|"rst"                        => ("≡", t.dim),
        "json"|"json5"|"jsonc"             => ("{", t.yellow),
        "yaml"|"yml"                       => ("{", t.green),
        "toml"                             => ("{", t.orange),
        "xml"                              => ("<", t.mid),
        "html"|"htm"                       => ("<", t.orange),
        "css"|"scss"|"sass"                => ("{", t.blue),
        "svg"                              => ("<", t.teal),
        "sh"|"bash"|"zsh"|"fish"           => ("$", t.green),
        "env"                              => ("#", t.green),
        "png"|"jpg"|"jpeg"|"gif"|"webp"|"bmp"|"ico" => ("□", t.teal),
        "pdf"                              => ("□", t.red),
        "zip"|"tar"|"gz"|"bz2"|"xz"|"7z"  => ("□", t.mid),
        "mp4"|"mov"|"mkv"|"webm"           => ("▷", t.mid),
        "mp3"|"wav"|"ogg"|"flac"           => ("♪", t.mid),
        "wasm"                             => ("■", t.purple),
        "proto"|"sql"|"nix"                => ("◈", t.cyan),
        "tf"|"tfvars"                      => ("◈", t.purple),
        "graphql"|"gql"                    => ("◈", t.pink),
        _                                  => ("·", t.dim),
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn popup_block(title: &'static str, t: &Theme) -> Block<'static> {
    Block::default()
        .title(Line::from(vec![
            Span::styled(title, Style::default().fg(t.mid)),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent))
        .style(Style::default().bg(t.bg2))
}

fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    Rect {
        x: (area.width.saturating_sub(w)) / 2,
        y: (area.height.saturating_sub(h)) / 2,
        width: w, height: h,
    }
}

fn provider_color(label: &str, t: &Theme) -> Color {
    match label {
        "github"   => t.mid,
        "gitlab"   => t.orange,
        "codeberg" => t.blue,
        "gitea"    => t.green,
        _          => t.dim,
    }
}

fn key_span(s: &'static str, col: Color) -> Span<'static> {
    Span::styled(s, Style::default().fg(col).add_modifier(Modifier::BOLD))
}
fn hint_span(s: &'static str, col: Color) -> Span<'static> {
    Span::styled(s, Style::default().fg(col))
}
