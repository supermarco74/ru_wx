//! Bridge MCP (stdio ↔ HTTP locale) — cfr. `PIANO_DEBUG_INSPECTOR.md` §8.
//!
//! Processo separato che parla **MCP su stdio** (JSON-RPC 2.0, un messaggio
//! per riga) e inoltra le richieste all'inspector HTTP dell'app `ru_wx`
//! (`127.0.0.1:PORTA`). Non duplica logica: ogni tool è una chiamata HTTP.
//!
//! Uso:
//! ```bash
//! ru_wx_mcp --port 7373
//! ```
//!
//! Configurazione lato IDE (Cursor / Claude Desktop):
//! ```json
//! {
//!   "mcpServers": {
//!     "ru_wx_inspector": {
//!       "command": "/percorso/ru_wx_mcp",
//!       "args": ["--port", "7373"]
//!     }
//!   }
//! }
//! ```

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde_json::{json, Value};

const HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 7373;
const PROTOCOL_VERSION: &str = "2024-11-05";
const TIMEOUT: Duration = Duration::from_secs(10);

fn main() {
    let port = parse_port();
    let stdin = std::io::stdin();
    let reader = BufReader::new(stdin.lock());
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let message: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if let Some(response) = handle(&message, port) {
            let _ = writeln!(out, "{response}");
            let _ = out.flush();
        }
    }
}

/// Legge `--port N` (o `-p N`) dalla riga di comando.
fn parse_port() -> u16 {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--port" || args[i] == "-p" {
            if let Some(value) = args.get(i + 1) {
                if let Ok(port) = value.parse() {
                    return port;
                }
            }
        }
        i += 1;
    }
    DEFAULT_PORT
}

// ── Dispatch JSON-RPC ───────────────────────────────────────────────

fn handle(message: &Value, port: u16) -> Option<Value> {
    let method = message.get("method")?.as_str()?;
    let id = message.get("id").cloned();
    let is_notification = id.is_none() || id == Some(Value::Null);

    match method {
        "initialize" => Some(result(id, initialize_result())),
        "notifications/initialized" | "initialized" => None,
        "ping" => Some(result(id, json!({}))),
        "tools/list" => Some(result(id, json!({ "tools": tool_schemas() }))),
        "tools/call" => {
            let (content, is_error) = call_tool(message, port);
            let mut payload = json!({ "content": content });
            if is_error {
                payload["isError"] = json!(true);
            }
            Some(result(id, payload))
        }
        _ => {
            if is_notification {
                None
            } else {
                Some(error(
                    id,
                    -32601,
                    &format!("metodo non supportato: {method}"),
                ))
            }
        }
    }
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "ru_wx_inspector", "version": env!("CARGO_PKG_VERSION") },
    })
}

fn result(id: Option<Value>, value: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": value })
}

fn error(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": { "code": code, "message": message },
    })
}

// ── Tool MCP (cfr. §8) ──────────────────────────────────────────────

fn tool_schemas() -> Vec<Value> {
    vec![
        json!({
            "name": "get_ui_tree",
            "description": "Restituisce l'albero dei componenti della finestra (tipo, testo, stato, colori, posizione reale).",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
        }),
        json!({
            "name": "find_element",
            "description": "Trova gli elementi per id/ruolo/testo/percorso. Restituisce id e bounds.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "handle esatto del nodo" },
                    "role": { "type": "string", "description": "button,label,input,check,choice,list,table,scroll,window" },
                    "text": { "type": "string", "description": "substring sul testo visibile (case-insensitive)" },
                    "path": { "type": "string", "description": "percorso posizionale, es. 0/2/1" },
                },
                "additionalProperties": false,
            },
        }),
        json!({
            "name": "get_window_info",
            "description": "Metadati della finestra: titolo, rect su schermo, area client, schermo e DPI.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
        }),
        json!({
            "name": "interact_with_element",
            "description": "Inietta un'azione su un elemento (click, focus, set_text, select).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "description": "click | focus | set_text | select" },
                    "id": { "type": "string", "description": "handle del nodo bersaglio" },
                    "text": { "type": "string", "description": "testo per set_text" },
                    "key": { "type": "string", "description": "indice per select" },
                },
                "required": ["action", "id"],
                "additionalProperties": false,
            },
        }),
        json!({
            "name": "take_screenshot",
            "description": "Screenshot PNG dell'intera finestra, codificato in base64.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string", "description": "opzionale: ritaglia sul nodo" } },
                "additionalProperties": false,
            },
        }),
        json!({
            "name": "get_visual_snapshot",
            "description": "Screenshot PNG ritagliato su un elemento (per id), codificato in base64.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string", "description": "handle del nodo" } },
                "required": ["id"],
                "additionalProperties": false,
            },
        }),
    ]
}

fn call_tool(message: &Value, port: u16) -> (Value, bool) {
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    match name {
        "get_ui_tree" => match request_json(port, "/tree") {
            Ok(value) => (text_content(&pretty(&value)), false),
            Err(err) => (text_content(&err), true),
        },
        "get_window_info" => match request_json(port, "/window") {
            Ok(value) => (text_content(&pretty(&value)), false),
            Err(err) => (text_content(&err), true),
        },
        "find_element" => {
            let query = selector_query(&args);
            let path = if query.is_empty() {
                "/find".to_string()
            } else {
                format!("/find?{query}")
            };
            match request_json(port, &path) {
                Ok(value) => (text_content(&pretty(&value)), false),
                Err(err) => (text_content(&err), true),
            }
        }
        "interact_with_element" => {
            let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
            let body = json!({
                "action": action,
                "id": args.get("id").and_then(|v| v.as_str()),
                "text": args.get("text").and_then(|v| v.as_str()),
                "key": args.get("key").and_then(|v| v.as_str()),
            });
            let bytes = serde_json::to_vec(&body).unwrap_or_default();
            match http_request(port, "POST", "/event", Some(&bytes)) {
                Ok((status, _, body)) => {
                    let value = serde_json::from_slice::<Value>(&body)
                        .unwrap_or_else(|_| json!({ "ok": status < 400 }));
                    (text_content(&pretty(&value)), status >= 400)
                }
                Err(err) => (text_content(&err), true),
            }
        }
        "take_screenshot" | "get_visual_snapshot" => {
            let query = selector_query(&args);
            let path = if query.is_empty() {
                "/screenshot".to_string()
            } else {
                format!("/screenshot?{query}")
            };
            match http_request(port, "GET", &path, None) {
                Ok((200, content_type, body)) => (
                    json!([{
                        "type": "image",
                        "data": base64_encode(&body),
                        "mimeType": if content_type.is_empty() { "image/png".to_string() } else { content_type },
                    }]),
                    false,
                ),
                Ok((_, _, body)) => {
                    let text = String::from_utf8_lossy(&body).to_string();
                    (text_content(&format!("errore screenshot: {text}")), true)
                }
                Err(err) => (text_content(&err), true),
            }
        }
        other => (text_content(&format!("tool sconosciuto: {other}")), true),
    }
}

fn text_content(text: &str) -> Value {
    json!([{ "type": "text", "text": text }])
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

/// Costruisce la query `k=v&…` dai parametri di un selettore, con URL-encoding.
fn selector_query(args: &Value) -> String {
    let mut parts = Vec::new();
    for key in ["id", "role", "text", "path"] {
        if let Some(value) = args.get(key).and_then(|v| v.as_str()) {
            if !value.is_empty() {
                parts.push(format!("{key}={}", url_encode(value)));
            }
        }
    }
    parts.join("&")
}

// ── HTTP client minimale ────────────────────────────────────────────

fn request_json(port: u16, path: &str) -> Result<Value, String> {
    let (status, _, body) = http_request(port, "GET", path, None)?;
    let value: Value = serde_json::from_slice(&body)
        .map_err(|err| format!("risposta non JSON (status {status}): {err}"))?;
    if status >= 400 {
        return Err(format!("errore HTTP {status}: {}", pretty(&value)));
    }
    Ok(value)
}

fn http_request(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
) -> Result<(u16, String, Vec<u8>), String> {
    let mut stream = TcpStream::connect((HOST, port)).map_err(|err| {
        format!("connessione a {HOST}:{port} fallita ({err}); l'inspector è in esecuzione?")
    })?;
    let _ = stream.set_read_timeout(Some(TIMEOUT));
    let _ = stream.set_write_timeout(Some(TIMEOUT));

    let mut request =
        format!("{method} {path} HTTP/1.1\r\nHost: {HOST}:{port}\r\nConnection: close\r\n");
    if let Some(body) = body {
        request.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        ));
    }
    request.push_str("\r\n");

    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("invio richiesta fallito: {err}"))?;
    if let Some(body) = body {
        stream
            .write_all(body)
            .map_err(|err| format!("invio body fallito: {err}"))?;
    }

    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|err| format!("lettura risposta fallita: {err}"))?;

    let header_end = find_subsequence(&raw, b"\r\n\r\n")
        .ok_or_else(|| "risposta HTTP malformata".to_string())?;
    let head = String::from_utf8_lossy(&raw[..header_end]).to_string();
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or(0);
    let content_type = head
        .lines()
        .find_map(|line| {
            let lower = line.to_ascii_lowercase();
            lower
                .strip_prefix("content-type:")
                .map(|value| value.trim().to_string())
        })
        .unwrap_or_default();
    let body = raw[header_end + 4..].to_vec();
    Ok((status, content_type, body))
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

// ── Base64 (RFC 4648) — nessuna dipendenza esterna ──────────────────

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn selector_query_encodes_and_skips_empty() {
        let args = json!({ "role": "button", "text": "Click Me!", "id": "" });
        let query = selector_query(&args);
        assert!(query.contains("role=button"));
        assert!(query.contains("text=Click+Me%21"));
        assert!(!query.contains("id="));
    }
}
