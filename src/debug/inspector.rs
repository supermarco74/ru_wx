//! Debug Inspector — **Livello 1** (cfr. `PIANO_DEBUG_INSPECTOR.md` §3, §5).
//!
//! Server HTTP/JSON minimale su `127.0.0.1`, scritto a mano su
//! `std::net` (nessun runtime async). Il thread server non tocca mai la
//! GUI: accoda un `Job` e sveglia il thread UI con
//! [`Frame::post_raw_message_to_hwnd`](crate::Frame::post_raw_message_to_hwnd); il thread UI esegue il job
//! (siamo sul thread giusto per AppKit/Win32) e risponde sul canale.
//!
//! In questo file non si importano `objc2` / `windows-sys`: le letture
//! passano da [`crate::platform::inspector`] (Livello 3), che a sua
//! volta usa i getter pubblici dei widget (Livello 2).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU16, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use crate::platform::inspector::{self, InjectedEvent, Selector};
use crate::platform::NativeHandle;

/// Codice messaggio custom usato per svegliare il thread UI.
/// Scelto in alto nel range `WM_USER..0x7FFF` per non collidere con i
/// messaggi dell'applicazione.
pub const RAW_MSG_INSPECTOR: u32 = 0x7FE0;

/// Timeout di attesa della risposta del thread UI.
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

/// Configurazione dell'inspector.
#[derive(Clone, Debug)]
pub struct InspectorConfig {
    pub host: String,
    pub port: u16,
    pub allow_events: bool,
}

impl Default for InspectorConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 7373,
            allow_events: true,
        }
    }
}

// ── Stato globale del server ────────────────────────────────────────

static RUNNING: AtomicBool = AtomicBool::new(false);
static ROOT: AtomicIsize = AtomicIsize::new(0);
static PORT: AtomicU16 = AtomicU16::new(0);
static ALLOW_EVENTS: AtomicBool = AtomicBool::new(true);
static JOBS: Mutex<Vec<Job>> = Mutex::new(Vec::new());

/// Chiusura che sveglia il thread UI in uso. Il backend la installa in
/// [`start_with`] (Win32: post di `RAW_MSG_INSPECTOR` sulla finestra
/// message-only dedicata, o in fallback sulla finestra radice) o in
/// [`start_xaml`] (XAML: dispatch sul `DispatcherQueue` del reactor).
/// Vedi §5.
type UiWake = Arc<dyn Fn() + Send + Sync>;
static WAKE: Mutex<Option<UiWake>> = Mutex::new(None);

struct Job {
    req: JobRequest,
    resp: Sender<Reply>,
}

struct Reply {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

#[derive(Debug)]
enum JobRequest {
    Health,
    Window,
    Tree,
    Find(Selector),
    Screenshot(Selector),
    Event(InjectedEvent),
    BadRequest(String),
    NotFound(String),
}

// ── API pubblica ────────────────────────────────────────────────────

/// Avvia l'inspector con la configurazione di default. Ritorna la porta
/// effettivamente in ascolto, oppure `None` se il bind fallisce.
pub fn start(frame: &crate::window::frame::Frame) -> Option<u16> {
    start_with(frame, InspectorConfig::default())
}

/// Avvia l'inspector con una configurazione esplicita.
pub fn start_with(frame: &crate::window::frame::Frame, cfg: InspectorConfig) -> Option<u16> {
    let root = frame.hwnd();
    // Win32: il thread UI si sveglia con un `RAW_MSG_INSPECTOR`.
    // L'handler registrato sulla frame copre il caso "normale", ma la
    // sua `Vec` viene rimossa dalla mappa mentre i callback girano:
    // se un job apre un loop modale al proprio interno (`show_tip`),
    // i post successivi troverebbero la lista vuota. Per questo il
    // wake primario punta alla **finestra message-only** dedicata
    // (`Frame::inspector_wake_window`), la cui wnd_proc richiama
    // `drain_jobs` direttamente ed è quindi rientrante.
    frame.register_raw_message_handler(RAW_MSG_INSPECTOR, |_w, _l| drain_jobs());
    #[cfg(target_os = "windows")]
    let wake_root = crate::window::frame::Frame::inspector_wake_window().unwrap_or(root);
    #[cfg(not(target_os = "windows"))]
    let wake_root = root;
    let wake: UiWake = Arc::new(move || {
        crate::window::frame::Frame::post_raw_message_to_hwnd(wake_root, RAW_MSG_INSPECTOR, 0, 0);
    });
    start_common(root, cfg, wake)
}

/// Avvia l'inspector su un frame del backend **XAML/reactor**.
///
/// I widget XAML non hanno un `HWND`: il "handle" radice è il `frame_id` e
/// il thread UI si sveglia accodando un job sul `DispatcherQueue` WinUI
/// (ottenuto dal reactor). Va chiamata **sul thread UI**, dopo che WinUI ha
/// avviato la propria coda di dispatch (§5, §10.1).
#[cfg(all(target_os = "windows", feature = "winui3_xaml"))]
pub fn start_xaml(frame_id: u64) -> Option<u16> {
    let dispatcher = windows_reactor::WinUIDispatcher::for_current_thread().ok()?;
    let marshaller = dispatcher.marshaller();
    // Il `DispatcherQueue` è agile: il thread server può accodarvi `drain_jobs`,
    // che così gira sul thread UI (dove vivono le `Element` e il registro §10.1).
    let wake: UiWake = Arc::new(move || {
        let _ = marshaller.dispatch(drain_jobs);
    });
    start_common(frame_id as NativeHandle, InspectorConfig::default(), wake)
}

/// Parte comune di [`start_with`] / [`start_xaml`]: registra radice e
/// wake-up, apre il listener e avvia il thread server.
fn start_common(root: NativeHandle, cfg: InspectorConfig, wake: UiWake) -> Option<u16> {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return Some(PORT.load(Ordering::SeqCst));
    }

    ROOT.store(root, Ordering::SeqCst);
    ALLOW_EVENTS.store(cfg.allow_events, Ordering::SeqCst);
    if let Ok(mut w) = WAKE.lock() {
        *w = Some(wake);
    }

    let listener = match TcpListener::bind((cfg.host.as_str(), cfg.port)) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!(
                "[ru_wx inspector] bind su {}:{} fallito: {err}",
                cfg.host, cfg.port
            );
            RUNNING.store(false, Ordering::SeqCst);
            return None;
        }
    };
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(_) => cfg.port,
    };
    PORT.store(port, Ordering::SeqCst);

    let spawned = std::thread::Builder::new()
        .name("ru_wx-inspector".to_string())
        .spawn(move || serve(listener));

    if spawned.is_err() {
        RUNNING.store(false, Ordering::SeqCst);
        return None;
    }

    println!(
        "[ru_wx inspector] in ascolto su http://{}:{}",
        cfg.host, port
    );
    Some(port)
}

/// Ferma il server (chiude il thread di ascolto alla prossima richiesta).
pub fn stop() {
    if !RUNNING.swap(false, Ordering::SeqCst) {
        return;
    }
    // Sveglia l'`accept()` bloccante con una connessione usa-e-getta.
    let port = PORT.load(Ordering::SeqCst);
    if port != 0 {
        let _ = TcpStream::connect(("127.0.0.1", port));
    }
    PORT.store(0, Ordering::SeqCst);
}

/// `true` se il server è in ascolto.
pub fn is_running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

/// Porta in ascolto (`0` se non attivo).
pub fn port() -> u16 {
    PORT.load(Ordering::SeqCst)
}

// ── Lato thread UI ──────────────────────────────────────────────────

/// Esegue i job accodati dal thread server. **Va chiamata sul thread UI.**
pub fn drain_jobs() {
    let jobs: Vec<Job> = match JOBS.lock() {
        Ok(mut queue) => std::mem::take(&mut *queue),
        Err(_) => return,
    };
    for job in jobs {
        let reply = handle(job.req);
        let _ = job.resp.send(reply);
    }
}

fn handle(req: JobRequest) -> Reply {
    let root: NativeHandle = ROOT.load(Ordering::SeqCst);
    match req {
        JobRequest::Health => json_reply(
            200,
            json!({
                "ok": true,
                "platform": std::env::consts::OS,
                "version": env!("CARGO_PKG_VERSION"),
                "pid": std::process::id(),
                "allow_events": ALLOW_EVENTS.load(Ordering::SeqCst),
            }),
        ),
        JobRequest::Window => {
            let info = inspector::window_info(root);
            json_reply(200, serde_json::to_value(info).unwrap_or(Value::Null))
        }
        JobRequest::Tree => {
            let tree = inspector::dump_tree(root);
            json_reply(200, serde_json::to_value(tree).unwrap_or(Value::Null))
        }
        JobRequest::Find(selector) => {
            let nodes = inspector::find_nodes(root, &selector);
            json_reply(
                200,
                json!({ "ok": true, "count": nodes.len(), "nodes": nodes }),
            )
        }
        JobRequest::Screenshot(selector) => match inspector::node_screenshot(root, &selector) {
            Ok(png) => Reply {
                status: 200,
                content_type: "image/png",
                body: png,
            },
            Err(message) => error_reply(400, "screenshot_failed", &message),
        },
        JobRequest::Event(event) => {
            if !events_allowed() {
                return error_reply(403, "events_disabled", "le azioni sono disattivate");
            }
            match inspector::inject_event(root, &event) {
                Ok(()) => json_reply(200, json!({ "ok": true, "action": event.action })),
                Err(message) => error_reply(400, "action_failed", &message),
            }
        }
        JobRequest::BadRequest(message) => error_reply(400, "bad_request", &message),
        JobRequest::NotFound(path) => {
            error_reply(404, "not_found", &format!("percorso sconosciuto: {path}"))
        }
    }
}

fn json_reply(status: u16, value: Value) -> Reply {
    Reply {
        status,
        content_type: "application/json",
        body: serde_json::to_vec(&value).unwrap_or_else(|_| b"null".to_vec()),
    }
}

fn error_reply(status: u16, code: &str, message: &str) -> Reply {
    json_reply(
        status,
        json!({ "ok": false, "error": { "code": code, "message": message } }),
    )
}

// ── Lato thread server ──────────────────────────────────────────────

fn serve(listener: TcpListener) {
    for stream in listener.incoming() {
        if !RUNNING.load(Ordering::SeqCst) {
            break;
        }
        if let Ok(mut stream) = stream {
            let _ = handle_connection(&mut stream);
        }
    }
}

fn handle_connection(stream: &mut TcpStream) -> std::io::Result<()> {
    let _ = stream.set_read_timeout(Some(REPLY_TIMEOUT));
    let (method, target, body) = match read_request(stream) {
        Ok(parts) => parts,
        Err(_) => return Ok(()),
    };

    let request = route(&method, &target, &body);
    let reply = match request {
        Some(req) => dispatch(req),
        None => error_reply(404, "not_found", &format!("percorso sconosciuto: {target}")),
    };
    write_response(stream, reply)
}

/// Accoda il job e attende la risposta del thread UI.
fn dispatch(req: JobRequest) -> Reply {
    if !RUNNING.load(Ordering::SeqCst) {
        return error_reply(503, "not_running", "inspector non attivo");
    }
    let (tx, rx) = mpsc::channel::<Reply>();
    {
        let mut queue = match JOBS.lock() {
            Ok(queue) => queue,
            Err(_) => return error_reply(500, "internal", "coda job avvelenata"),
        };
        queue.push(Job { req, resp: tx });
    }
    wake_ui();
    match rx.recv_timeout(REPLY_TIMEOUT) {
        Ok(reply) => reply,
        Err(_) => error_reply(504, "timeout", "il thread UI non ha risposto in tempo"),
    }
}

/// Sveglia il thread UI tramite la chiusura installata dal backend.
fn wake_ui() {
    let wake = WAKE.lock().ok().and_then(|w| w.as_ref().cloned());
    if let Some(wake) = wake {
        wake();
    }
}

fn route(method: &str, target: &str, body: &[u8]) -> Option<JobRequest> {
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    };
    match (method, path) {
        ("GET", "/health") => Some(JobRequest::Health),
        ("GET", "/window") => Some(JobRequest::Window),
        ("GET", "/tree") => Some(JobRequest::Tree),
        ("GET", "/find") => Some(JobRequest::Find(parse_selector(query))),
        ("GET", "/screenshot") => Some(JobRequest::Screenshot(parse_selector(query))),
        ("POST", "/event") => match serde_json::from_slice::<InjectedEvent>(body) {
            Ok(event) => Some(JobRequest::Event(event)),
            Err(err) => Some(JobRequest::BadRequest(format!("JSON non valido: {err}"))),
        },
        _ => Some(JobRequest::NotFound(path.to_string())),
    }
}

fn parse_selector(query: &str) -> Selector {
    let mut selector = Selector::default();
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        let value = url_decode(value);
        match key {
            "id" => selector.id = Some(value),
            "role" => selector.role = Some(value),
            "text" => selector.text = Some(value),
            "path" => selector.path = Some(value),
            _ => {}
        }
    }
    selector
}

// ── HTTP minimale ───────────────────────────────────────────────────

fn read_request(stream: &mut TcpStream) -> std::io::Result<(String, String, Vec<u8>)> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "connessione chiusa prima degli header",
            ));
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_subsequence(&buf, b"\r\n\r\n") {
            break pos + 4;
        }
        if buf.len() > 1 << 20 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "header troppo lunghi",
            ));
        }
    };

    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or("/").to_string();

    let mut content_length = 0usize;
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(value) = lower.strip_prefix("content-length:") {
            content_length = value.trim().parse().unwrap_or(0);
        }
    }

    let mut body = buf[header_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(content_length);
    Ok((method, target, body))
}

fn write_response(stream: &mut TcpStream, reply: Reply) -> std::io::Result<()> {
    let status_text = match reply.status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "OK",
    };
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        reply.status,
        status_text,
        reply.content_type,
        reply.body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(&reply.body)?;
    stream.flush()
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn url_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `true` se le azioni (`POST /event`) sono accettate.
fn events_allowed() -> bool {
    ALLOW_EVENTS.load(Ordering::SeqCst)
}
