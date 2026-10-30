#![windows_subsystem = "windows"]

use std::{
    fs,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, SystemTime},
};

use pulldown_cmark::{Options, Parser, html};
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{Icon, WindowBuilder},
};
use wry::{WebContext, WebViewBuilder};

const DEFAULT_ROOT: &str = r"D:\Bryan\Dev\Docs";
const DATA_DIR: &str = r"D:\Bryan\Dev\Caches\mdview";
const SKIP_DIRS: [&str; 3] = ["node_modules", "target", ".git"];
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

enum Msg {
    Ipc(String),
    Tick,
}

fn is_md(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

fn scan(dir: &Path, root: &Path, out: &mut Vec<String>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            let skip = p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| SKIP_DIRS.contains(&n));
            if !skip {
                scan(&p, root, out);
            }
        } else if is_md(&p) {
            if let Ok(rel) = p.strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
}

fn list_files(root: &Path) -> Vec<String> {
    let mut v = Vec::new();
    scan(root, root, &mut v);
    v.sort_by_key(|s| s.to_lowercase());
    v
}

fn mtime(p: &Path) -> Option<SystemTime> {
    fs::metadata(p).and_then(|m| m.modified()).ok()
}

fn render(p: &Path) -> String {
    match fs::read(p) {
        Ok(b) => {
            let text = String::from_utf8_lossy(&b);
            let opts = Options::ENABLE_TABLES
                | Options::ENABLE_FOOTNOTES
                | Options::ENABLE_STRIKETHROUGH
                | Options::ENABLE_TASKLISTS;
            let mut out = String::new();
            html::push_html(&mut out, Parser::new_ext(&text, opts));
            out
        }
        Err(e) => format!("<p class=\"empty\">Não consegui abrir o arquivo: {e}</p>"),
    }
}

fn open_external(url: &str) {
    let _ = Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

fn main() {
    let arg = std::env::args_os().nth(1).map(PathBuf::from);
    let arg = arg.and_then(|p| std::path::absolute(p).ok());
    let (root, mut cur) = match arg {
        Some(p) if p.is_file() => (p.parent().map(Path::to_path_buf).unwrap_or_default(), Some(p)),
        Some(p) if p.is_dir() => (p, None),
        _ => (PathBuf::from(DEFAULT_ROOT), None),
    };
    let mut files = list_files(&root);
    let mut cur_mtime: Option<SystemTime> = None;

    let event_loop = EventLoopBuilder::<Msg>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("mdv")
        .with_window_icon(Icon::from_rgba(include_bytes!("../assets/icon-64.rgba").to_vec(), 64, 64).ok())
        .with_inner_size(LogicalSize::new(1100.0, 780.0))
        .build(&event_loop)
        .expect("janela");

    let mut ctx = WebContext::new(Some(PathBuf::from(DATA_DIR)));
    let ipc_proxy = proxy.clone();
    let nav_proxy = proxy.clone();
    let webview = WebViewBuilder::new_with_web_context(&mut ctx)
        .with_html(include_str!("page.html"))
        .with_ipc_handler(move |req| {
            let _ = ipc_proxy.send_event(Msg::Ipc(req.into_body()));
        })
        .with_navigation_handler(move |url| {
            // só a página embutida navega; o resto vai pro navegador
            if url.starts_with("data:") || url.starts_with("about:") {
                true
            } else {
                let _ = nav_proxy.send_event(Msg::Ipc(format!("ext:{url}")));
                false
            }
        })
        .build(&window)
        .expect("webview");

    thread::spawn(move || {
        while proxy.send_event(Msg::Tick).is_ok() {
            thread::sleep(Duration::from_millis(700));
        }
    });

    let rel = |root: &Path, p: &Path| {
        p.strip_prefix(root)
            .map(|r| r.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default()
    };
    let js = |s: &str| serde_json::to_string(s).unwrap_or_default();

    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *flow = ControlFlow::Exit,
            Event::UserEvent(Msg::Tick) => {
                let now = list_files(&root);
                if now != files {
                    files = now;
                    let list = serde_json::to_string(&files).unwrap_or_default();
                    let c = cur.as_deref().map(|p| rel(&root, p)).unwrap_or_default();
                    let _ = webview.evaluate_script(&format!("setList({list},{})", js(&c)));
                }
                if let Some(p) = &cur {
                    let m = mtime(p);
                    if m != cur_mtime {
                        cur_mtime = m;
                        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        let _ = webview.evaluate_script(&format!("setContent({},{})", js(&render(p)), js(&name)));
                    }
                }
            }
            Event::UserEvent(Msg::Ipc(m)) => {
                let (cmd, arg) = m.split_once(':').unwrap_or((m.as_str(), ""));
                let target = match cmd {
                    "open" => Some(root.join(arg)),
                    "link" => {
                        let href = arg.split('#').next().unwrap_or("");
                        cur.as_deref()
                            .and_then(Path::parent)
                            .map(|d| d.join(href.replace('/', "\\")))
                            .filter(|p| is_md(p) && p.is_file())
                    }
                    "ext" => {
                        open_external(arg);
                        None
                    }
                    "ready" | "refresh" => {
                        files = list_files(&root);
                        cur_mtime = None; // força re-render no próximo Tick
                        if cmd == "ready" && cur.is_none() {
                            cur = files.first().map(|f| root.join(f));
                        }
                        None
                    }
                    _ => None,
                };
                if let Some(p) = target {
                    cur = Some(p);
                    cur_mtime = None;
                }
                if matches!(cmd, "open" | "link" | "ready" | "refresh") {
                    let list = serde_json::to_string(&files).unwrap_or_default();
                    let c = cur.as_deref().map(|p| rel(&root, p)).unwrap_or_default();
                    let _ = webview.evaluate_script(&format!("setList({list},{})", js(&c)));
                    if let Some(p) = &cur {
                        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        window.set_title(&format!("{name} — mdv"));
                        cur_mtime = mtime(p);
                        let _ = webview.evaluate_script(&format!("setContent({},{})", js(&render(p)), js(&name)));
                    }
                }
            }
            _ => {}
        }
    });
}
