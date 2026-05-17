use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use serde_json;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct StyledLine {
    text: String,
    color: Option<Color>,
}

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

struct TuiState {
    output_rows: Vec<StyledLine>,
    input_buffer: String,
    scroll_offset: usize,
    input_scroll: usize,
    processing: bool,
    done: bool,
    demo: bool,
    stream_buf: String,
    spinner_idx: usize,
    /// 当前会话 ID
    session_id: Option<String>,
    /// 历史消息（Anthropic 格式）
    history_messages: Vec<serde_json::Value>,
}

impl TuiState {
    fn new(demo: bool) -> Self {
        let mut s = Self {
            output_rows: Vec::new(),
            input_buffer: String::new(),
            scroll_offset: usize::MAX,
            input_scroll: 0,
            processing: false,
            done: false,
            demo,
            stream_buf: String::new(),
            spinner_idx: 0,
            session_id: None,
            history_messages: Vec::new(),
        };
        s.push_line("Agent TUI ready. Type /help for commands.", None);
        if demo {
            s.push_line("[Demo mode] No API calls will be made.", Some(Color::DarkGray));
            s.push_line("Try: type a message, or use /help to see commands.", Some(Color::DarkGray));
        }
        s
    }

    /// 追加一行带颜色的文本。
    fn push_line(&mut self, text: &str, color: Option<Color>) {
        if !self.stream_buf.is_empty() {
            // 如果有未刷新的流式内容，先作为一行推入
            self.output_rows.push(StyledLine {
                text: std::mem::take(&mut self.stream_buf),
                color: None,
            });
        }
        self.output_rows.push(StyledLine {
            text: text.to_string(),
            color,
        });
    }

    /// 追加普通输出行。
    fn push_output(&mut self, text: &str) {
        self.push_line(text, None);
    }

    /// 追加命令输出行（`#  ` 前缀 + 蓝色）。
    fn push_cmd_output(&mut self, text: &str) {
        self.push_line(text, Some(Color::Cyan));
    }

    /// 追加用户输入行（`> ` 前缀 + 灰色）。
    fn push_user_input(&mut self, text: &str) {
        self.push_line(text, Some(Color::DarkGray));
    }

    /// 追加错误行（红色）。
    fn push_error(&mut self, text: &str) {
        self.push_line(text, Some(Color::Red));
    }

    /// 追加回显行（绿色）。
    fn push_echo(&mut self, text: &str) {
        self.push_line(text, Some(Color::Green));
    }

    /// 流式 token：累积到 stream_buf，下次 push_line 时刷入
    fn push_token(&mut self, token: &str) {
        self.stream_buf.push_str(token);
    }

    /// 构建渲染文本（从 output_rows + stream_buf）。
    fn render_text(&self) -> Text<'static> {
        let mut lines: Vec<Line<'static>> = self
            .output_rows
            .iter()
            .map(|r| {
                if let Some(color) = r.color {
                    Line::from(vec![Span::styled(r.text.clone(), Style::default().fg(color))])
                } else {
                    Line::from(r.text.clone())
                }
            })
            .collect();
        // 如果有累积的流式内容，追加为一行
        if !self.stream_buf.is_empty() {
            lines.push(Line::from(self.stream_buf.clone()));
        }
        Text::from(lines)
    }

    /// 获取显示行数（含流式）。
    fn display_lines(&self) -> usize {
        let base = self.output_rows.len();
        if self.stream_buf.is_empty() { base } else { base + 1 }
    }
}

fn clamp_scroll(scroll: usize, total_lines: usize, panel_height: usize) -> usize {
    if total_lines <= panel_height {
        return 0;
    }
    let max_scroll = total_lines - panel_height;
    scroll.min(max_scroll)
}

pub async fn run_tui(demo: bool, db: Option<rusqlite::Connection>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = TuiState::new(demo);
    let db = db.map(|c| Arc::new(Mutex::new(c)));
    let token_buffer: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

    loop {
        let term_size = terminal.size()?;
        // panel_height 与 Percentage(84) 布局一致，避免终端越大误差越大
        let avail = term_size.height.saturating_sub(2) as usize;
        let output_h = (avail * 84) / 100;
        let panel_height = output_h.saturating_sub(2); // 6 + 1 for status bar

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Percentage(84),
                    Constraint::Length(4),
                    Constraint::Length(1),
                ])
                .split(f.size());

            // ── 输出面板 ──
            let output_title = if state.processing {
                "Output (Agent is thinking...)"
            } else {
                "Output"
            };
            let total_lines = state.display_lines();
            let scroll = clamp_scroll(state.scroll_offset, total_lines, panel_height);
            state.scroll_offset = scroll;

            // 输入框滚动：内部可用 2 行（Height(4) - 边框 2）
            let input_visible = (chunks[1].height as usize).saturating_sub(2);
            let input_total = state.input_buffer.lines().count() + 1;
            state.input_scroll = clamp_scroll(state.input_scroll, input_total, input_visible);

            let output = Paragraph::new(state.render_text())
                .scroll((scroll as u16, 0))
                .wrap(Wrap { trim: false })
                .block(Block::default().title(output_title).borders(Borders::ALL));
            f.render_widget(output, chunks[0]);

            // ── 输入框 ──
            let input_title = if state.processing {
                "Agent is thinking..."
            } else {
                "Input (Enter to send, Esc to quit)"
            };
            let input = Paragraph::new(state.input_buffer.clone())
                .scroll((state.input_scroll as u16, 0))
                .wrap(Wrap { trim: false })
                .block(Block::default().title(input_title).borders(Borders::ALL));
            f.render_widget(input, chunks[1]);

            // ── 状态栏 ──
            let mode_tag = if state.demo { "demo" } else { "normal" };

            // token 用量 + 上下文
            let usage = crate::tools::llm::get_token_usage();
            let ctx_pct = crate::tools::llm::context_usage_pct();

            let token_str = if usage.input_tokens > 0 || usage.output_tokens > 0 {
                let cache = match usage.cache_hit_rate() {
                    Some(r) => format!("cache {:.0}%", r * 100.0),
                    None => "no-cache".to_string(),
                };
                let ctx = match ctx_pct {
                    Some(p) => format!("ctx {:.0}%", p),
                    None => String::new(),
                };
                format!("tok i{}/o{}  {}  {}", usage.input_tokens, usage.output_tokens, cache, ctx)
            } else {
                String::new()
            };

            let spinner = if state.processing {
                SPINNER[state.spinner_idx]
            } else {
                ' '
            };
            let status = format!(
                " {} [{}]  {}/{}  {}",
                spinner, mode_tag, scroll, total_lines, token_str
            );
            let status_style = Style::default().fg(Color::DarkGray).bg(Color::Black);
            let status_bar = Paragraph::new(Line::from(vec![
                Span::styled(status, status_style),
            ]));
            f.render_widget(status_bar, chunks[2]);
        })?;

        // 流式 token
        if state.processing && !state.demo {
            let s = {
                let mut buf = token_buffer.lock().unwrap();
                if buf.is_empty() {
                    String::new()
                } else {
                    std::mem::take(&mut *buf)
                }
            };
            if !s.is_empty() {
                state.push_token(&s);
            }
        }

        if state.processing {
            state.spinner_idx = (state.spinner_idx + 1) % SPINNER.len();
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            if state.done {
                break;
            }
            continue;
        }

        // 键盘事件
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                        if state.input_buffer.is_empty() {
                            continue;
                        }
                        let task = std::mem::take(&mut state.input_buffer);

                        if task.starts_with('/') {
                            state.processing = true;
                            process_command(&task, &mut state, &db).await;
                            state.processing = false;
                        } else {
                            state.push_user_input(&format!("> {}", task));
                            state.processing = true;

                            // 记录用户消息
                            state.history_messages.push(serde_json::json!({
                                "role": "user",
                                "content": task,
                            }));

                            if state.demo {
                                tokio::time::sleep(Duration::from_millis(800)).await;
                                let resp = format!("(Demo Echo) You said: {}", task);
                                state.push_echo(&resp);
                                state.history_messages.push(serde_json::json!({
                                    "role": "assistant",
                                    "content": resp,
                                }));
                            } else {
                                let tb = token_buffer.clone();
                                crate::tools::llm::set_streaming_hook(Box::new(move |token| {
                                    tb.lock().unwrap().push_str(token);
                                }));
                                let history = if state.history_messages.is_empty() {
                                    None
                                } else {
                                    // 去掉最后一条 user 消息（它作为 task 传入了）
                                    let prefix = &state.history_messages[..state.history_messages.len() - 1];
                                    Some(prefix)
                                };
                                match crate::run_agent_once(&task, history).await {
                                    Ok(resp) => {
                                        state.push_output(&resp);
                                        state.history_messages.push(serde_json::json!({
                                            "role": "assistant",
                                            "content": resp,
                                        }));
                                    }
                                    Err(e) => state.push_error(&format!("(Error: {})", e)),
                                }
                                crate::tools::llm::clear_streaming_hook();
                                let rest = {
                                    let mut buf = token_buffer.lock().unwrap();
                                    std::mem::take(&mut *buf)
                                };
                                if !rest.is_empty() { state.push_token(&rest); }
                                if !state.stream_buf.is_empty() {
                                    let t = std::mem::take(&mut state.stream_buf);
                                    state.push_output(&t);
                                }
                            }
                            state.processing = false;
                        }
                        state.scroll_offset = usize::MAX;
                    }
                    KeyCode::PageUp => {
                        state.scroll_offset =
                            state.scroll_offset.saturating_sub(panel_height);
                    }
                    KeyCode::PageDown => {
                        state.scroll_offset =
                            state.scroll_offset.saturating_add(panel_height);
                    }
                    KeyCode::Home => state.scroll_offset = 0,
                    KeyCode::End => state.scroll_offset = usize::MAX,
                    KeyCode::Char(c) => {
                        state.input_buffer.push(c);
                        state.input_scroll = usize::MAX;
                    }
                    KeyCode::Backspace => {
                        state.input_buffer.pop();
                        state.input_scroll = usize::MAX;
                    }
                    _ => {}
                }
            }
        }

        if state.done {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

async fn process_command(
    cmd: &str,
    state: &mut TuiState,
    db: &Option<Arc<Mutex<rusqlite::Connection>>>,
) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let command = parts[0];

    match command {
        "/clear" | "/c" => state.output_rows.clear(),
        "/help" | "/?" | "/h" => {
            state.push_output("");
            state.push_cmd_output("Available commands:");
            state.push_cmd_output("/new, /n         - Start a new session");
            state.push_cmd_output("/clear, /c       - Clear output panel");
            state.push_cmd_output("/echo <text>     - Echo text back");
            state.push_cmd_output("/save, /s        - Save current session");
            state.push_cmd_output("/history, /hist  - List saved sessions");
            state.push_cmd_output("/load <N>        - Load and restore session N");
            state.push_cmd_output("/help, /?, /h    - Show this help");
            state.push_cmd_output("/quit, /q, /exit - Exit program");
            state.push_output("");
            if state.demo {
                state.push_cmd_output("(Demo mode: no API calls)");
            }
        }
        "/echo" | "/e" => {
            let text = parts[1..].join(" ");
            if text.is_empty() {
                state.push_cmd_output("Usage: /echo <text>");
            } else {
                state.push_echo(&format!("echo: {}", text));
            }
        }
        "/new" | "/n" => {
            state.output_rows.clear();
            state.history_messages.clear();
            state.session_id = None;
            state.push_output("Agent TUI ready. Type /help for commands.");
            state.push_cmd_output("New session started.");
        }
        "/save" | "/s" => {
            let conn = match db {
                Some(c) => c,
                None => { state.push_cmd_output("No database."); return; }
            };
            if state.history_messages.is_empty() {
                state.push_cmd_output("Nothing to save.");
                return;
            }
            let title = state
                .history_messages
                .iter()
                .find_map(|m| m["role"].as_str().and_then(|r| {
                    if r == "user" { m["content"].as_str().map(|s| s.to_string()) }
                    else { None }
                }))
                .unwrap_or_else(|| "(untitled)".to_string());
            let title = if title.len() > 60 { format!("{}...", &title[..57]) } else { title };

            let conn = conn.lock().unwrap();
            let sid = match &state.session_id {
                Some(id) => id.clone(),
                None => {
                    match crate::persistence::create_session(&conn, &title) {
                        Ok(id) => { state.session_id = Some(id.clone()); id }
                        Err(e) => { state.push_cmd_output(&format!("Save error: {}", e)); return; }
                    }
                }
            };
            if let Err(e) = crate::persistence::save_messages(&conn, &sid, &state.history_messages) {
                state.push_cmd_output(&format!("Save error: {}", e));
                return;
            }
            state.push_cmd_output(&format!("Session saved [{:.8}]: {}", sid, title));
        }
        "/history" | "/hist" => {
            let conn = match db {
                Some(c) => c,
                None => { state.push_cmd_output("No database."); return; }
            };
            let sessions = match crate::persistence::list_sessions(&conn.lock().unwrap()) {
                Ok(s) => s,
                Err(e) => { state.push_cmd_output(&format!("List error: {}", e)); return; }
            };
            if sessions.is_empty() {
                state.push_cmd_output("No saved sessions. Use /save.");
                return;
            }
            state.push_output("");
            state.push_cmd_output("Saved sessions:");
            for (i, (id, title, updated, count)) in sessions.iter().enumerate() {
                let date = if updated.len() >= 10 { &updated[..10] } else { updated };
                state.push_cmd_output(&format!(
                    "  [{}] {}  {}  ({} msgs)  {}",
                    i + 1, date, title, count, &id[..8]
                ));
            }
            state.push_output("");
            state.push_cmd_output("Use /load <N> to restore.");
        }
        "/load" | "/l" => {
            let conn = match db {
                Some(c) => c,
                None => { state.push_cmd_output("No database."); return; }
            };
            let idx: usize = match parts.get(1).and_then(|s| s.parse::<usize>().ok()) {
                Some(n) if n > 0 => n - 1,
                _ => { state.push_cmd_output("Usage: /load <N>"); return; }
            };
            let sessions = match crate::persistence::list_sessions(&conn.lock().unwrap()) {
                Ok(s) => s,
                Err(e) => { state.push_cmd_output(&format!("List error: {}", e)); return; }
            };
            let (sid, title, _updated, _) = match sessions.get(idx) {
                Some(s) => s.clone(),
                None => { state.push_cmd_output(&format!("Session #{} not found.", idx + 1)); return; }
            };
            let msgs = match crate::persistence::load_messages(&conn.lock().unwrap(), &sid) {
                Ok(m) => m,
                Err(e) => { state.push_cmd_output(&format!("Load error: {}", e)); return; }
            };
            if msgs.is_empty() {
                state.push_cmd_output("Session is empty.");
                return;
            }
            // 恢复历史消息（用于后续对话上下文）
            state.history_messages = msgs.clone();
            state.session_id = Some(sid);

            // 重建输出面板
            state.output_rows.clear();
            state.output_rows.push(StyledLine {
                text: format!("--- Loading session: {} ---", title),
                color: Some(Color::Cyan),
            });
            for msg in &msgs {
                let role = msg["role"].as_str().unwrap_or("");
                let content = match msg["content"].as_str() {
                    Some(s) => s.to_string(),
                    None => serde_json::to_string(&msg["content"]).unwrap_or_default(),
                };
                match role {
                    "user" => state.push_user_input(&format!("> {}", content)),
                    "assistant" => state.push_output(&content),
                    _ => state.push_output(&content),
                }
            }
            state.push_cmd_output(&format!("Session {} restored ({} msgs). You can continue chatting.", idx + 1, msgs.len()));
        }
        "/quit" | "/q" | "/exit" => state.done = true,
        _ => {
            state.push_cmd_output(&format!("Unknown: {}", command));
            state.push_cmd_output("Type /help to see available commands.");
        }
    }
}
