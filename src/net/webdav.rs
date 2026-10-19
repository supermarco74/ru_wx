//! Minimal WebDAV client (Nextcloud / ownCloud / generic servers).
//!
//! Supports the essential operations for browsing and moving files:
//!
//! * `PROPFIND` (Depth 1) → folder listing
//! * `GET` → download
//! * `PUT` → upload
//! * `DELETE` → delete
//! * `MKCOL` → create folder
//! * `MOVE` → rename / move
//!
//! Everything goes through `super::http_request`, so there are no
//! external HTTP/XML crates: the multistatus response is parsed with a
//! small string scanner covering the `href` / `resourcetype` /
//! `getcontentlength` tags.

use super::http_request;

/// Parts of an already-parsed WebDAV URL.
#[derive(Clone, Debug)]
pub struct UrlParts {
    pub secure: bool,
    pub host: String,
    pub port: u16,
    /// Base collection path, e.g. `/remote.php/dav/files/user/`.
    pub base_path: String,
}

/// A single entry returned by `PROPFIND`.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    /// Remote path relative to the collection (e.g. `/docs/file.txt`).
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    /// Last modification time (seconds since the Unix epoch), from the
    /// `getlastmodified` property. `None` when absent/unparsable.
    pub modified_secs: Option<u64>,
}

/// A WebDAV client authenticated with HTTP Basic auth.
#[derive(Clone, Debug)]
pub struct Client {
    pub url: UrlParts,
    pub username: String,
    pub password: String,
}

/// Split a URL (`http://` or `https://`) into [`UrlParts`].
pub fn parse_url(url: &str) -> Result<UrlParts, String> {
    let (secure, rest) = if let Some(r) = url.strip_prefix("https://") {
        (true, r)
    } else if let Some(r) = url.strip_prefix("http://") {
        (false, r)
    } else {
        return Err("URL must start with http:// or https://".into());
    };

    // Split host[:port] from the path.
    let (hostport, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };

    let (host, port) = if let Some(colon) = hostport.rfind(':') {
        let host = &hostport[..colon];
        let port_str = &hostport[colon + 1..];
        let port = port_str
            .parse::<u16>()
            .map_err(|_| "invalid port".to_string())?;
        (host.to_string(), port)
    } else {
        let default = if secure { 443 } else { 80 };
        (hostport.to_string(), default)
    };

    if host.is_empty() {
        return Err("missing host in URL".into());
    }

    // Normalize the base path: always ends with `/`.
    let mut base_path = if path.is_empty() { "/" } else { path }.to_string();
    if !base_path.ends_with('/') {
        base_path.push('/');
    }

    Ok(UrlParts {
        secure,
        host,
        port,
        base_path,
    })
}

impl Client {
    pub fn new(url: &str, username: &str, password: &str) -> Result<Client, String> {
        Ok(Client {
            url: parse_url(url)?,
            username: username.to_string(),
            password: password.to_string(),
        })
    }

    fn auth(&self) -> (String, String) {
        http_request::basic_auth_header(&self.username, &self.password)
    }

    /// Absolute URL path (relative to the collection) for `rel_path`.
    fn url_path(&self, rel_path: &str) -> String {
        let rel = rel_path.trim_start_matches('/');
        format!("{}{}", self.url.base_path, url_encode_path(rel))
    }

    /// List the contents of `rel_path` (folder).
    pub fn list(&self, rel_path: &str) -> Result<Vec<Entry>, String> {
        let path = self.url_path(rel_path);
        let body = br#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:">
  <d:prop>
    <d:resourcetype/>
    <d:getcontentlength/>
    <d:getlastmodified/>
  </d:prop>
</d:propfind>"#;
        let (status, resp) = http_request::request(
            "PROPFIND",
            &self.url.host,
            self.url.port,
            self.url.secure,
            &path,
            &[
                self.auth(),
                ("Depth".to_string(), "1".to_string()),
                (
                    "Content-Type".to_string(),
                    "application/xml; charset=utf-8".to_string(),
                ),
            ],
            body,
        )?;
        if !(200..300).contains(&status) {
            return Err(format!(
                "PROPFIND failed (HTTP {status}): {}",
                short_body(&resp)
            ));
        }
        parse_multistatus(&resp, &self.url.base_path)
    }

    /// Metadati di una singola risorsa (`rel_path`), via `PROPFIND`
    /// Depth 0. Ritorna `Ok(None)` se la risorsa non esiste (404).
    pub fn stat(&self, rel_path: &str) -> Result<Option<Entry>, String> {
        let path = self.url_path(rel_path);
        let body = br#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:">
  <d:prop>
    <d:resourcetype/>
    <d:getcontentlength/>
    <d:getlastmodified/>
  </d:prop>
</d:propfind>"#;
        let (status, resp) = http_request::request(
            "PROPFIND",
            &self.url.host,
            self.url.port,
            self.url.secure,
            &path,
            &[
                self.auth(),
                ("Depth".to_string(), "0".to_string()),
                (
                    "Content-Type".to_string(),
                    "application/xml; charset=utf-8".to_string(),
                ),
            ],
            body,
        )?;
        if status == 404 {
            return Ok(None);
        }
        if !(200..300).contains(&status) {
            return Err(format!(
                "PROPFIND failed (HTTP {status}): {}",
                short_body(&resp)
            ));
        }
        let text = String::from_utf8_lossy(&resp);
        let is_dir = contains_tag(&text, "collection");
        let size = xml_tag(&text, "getcontentlength")
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);
        let modified_secs =
            xml_tag(&text, "getlastmodified").and_then(|s| super::dates::parse_rfc1123(s.trim()));
        let name = rel_path
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("/")
            .to_string();
        Ok(Some(Entry {
            name,
            path: if rel_path.starts_with('/') {
                rel_path.to_string()
            } else {
                format!("/{rel_path}")
            },
            is_dir,
            size,
            modified_secs,
        }))
    }

    /// Download the contents of `rel_path`.
    pub fn get(&self, rel_path: &str) -> Result<Vec<u8>, String> {
        let path = self.url_path(rel_path);
        let (status, resp) = http_request::request(
            "GET",
            &self.url.host,
            self.url.port,
            self.url.secure,
            &path,
            &[self.auth()],
            &[],
        )?;
        if !(200..300).contains(&status) {
            return Err(format!("GET failed (HTTP {status})"));
        }
        Ok(resp)
    }

    /// Upload `data` to `rel_path` (creating / updating the file).
    pub fn put(&self, rel_path: &str, data: &[u8]) -> Result<(), String> {
        let path = self.url_path(rel_path);
        let (status, _resp) = http_request::request(
            "PUT",
            &self.url.host,
            self.url.port,
            self.url.secure,
            &path,
            &[
                self.auth(),
                (
                    "Content-Type".to_string(),
                    "application/octet-stream".to_string(),
                ),
            ],
            data,
        )?;
        // 200/201/204 = success.
        if !(200..300).contains(&status) {
            return Err(format!("PUT failed (HTTP {status})"));
        }
        Ok(())
    }

    /// Delete `rel_path` (file or folder).
    pub fn delete(&self, rel_path: &str) -> Result<(), String> {
        let path = self.url_path(rel_path);
        let (status, _resp) = http_request::request(
            "DELETE",
            &self.url.host,
            self.url.port,
            self.url.secure,
            &path,
            &[self.auth()],
            &[],
        )?;
        if !(200..300).contains(&status) {
            return Err(format!("DELETE failed (HTTP {status})"));
        }
        Ok(())
    }

    /// Create a folder `rel_path`.
    pub fn mkcol(&self, rel_path: &str) -> Result<(), String> {
        let path = self.url_path(rel_path);
        let (status, _resp) = http_request::request(
            "MKCOL",
            &self.url.host,
            self.url.port,
            self.url.secure,
            &path,
            &[self.auth()],
            &[],
        )?;
        // 201 = created, 405 = already exists (not an error for us).
        if status != 201 && status != 405 {
            return Err(format!("MKCOL failed (HTTP {status})"));
        }
        Ok(())
    }

    /// Move / rename `rel_path` to `new_rel_path` (MOVE).
    pub fn mv(&self, rel_path: &str, new_rel_path: &str) -> Result<(), String> {
        let source = self.url_path(rel_path);
        let destination = self.absolute_url(new_rel_path);
        let (status, _resp) = http_request::request(
            "MOVE",
            &self.url.host,
            self.url.port,
            self.url.secure,
            &source,
            &[
                self.auth(),
                ("Destination".to_string(), destination),
                ("Overwrite".to_string(), "T".to_string()),
            ],
            &[],
        )?;
        if !(200..300).contains(&status) {
            return Err(format!("MOVE failed (HTTP {status})"));
        }
        Ok(())
    }

    /// Absolute URL of `rel_path` (scheme + host[:port] + path).
    fn absolute_url(&self, rel_path: &str) -> String {
        let scheme = if self.url.secure { "https" } else { "http" };
        let path = self.url_path(rel_path);
        let default = if self.url.secure { 443 } else { 80 };
        if self.url.port == default {
            format!("{scheme}://{}{path}", self.url.host)
        } else {
            format!("{scheme}://{}:{}{path}", self.url.host, self.url.port)
        }
    }
}

/// Convert a body into a short description (first non-empty line).
fn short_body(body: &[u8]) -> String {
    String::from_utf8_lossy(body)
        .lines()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .unwrap_or_default()
}

/// Minimal percent-encoding for WebDAV path segments.
fn url_encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for b in path.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

/// Extract the text content of an XML tag, accepting any namespace
/// prefix (`<name>`, `<d:name>`, `<D:name>`, `<ns0:name>`, …).
/// Returns `None` if the tag is absent.
fn xml_tag(xml: &str, name: &str) -> Option<String> {
    let mut search = 0usize;
    while let Some(lt) = xml[search..].find('<') {
        let start = search + lt;
        let after = &xml[start + 1..];
        // Nome dell'elemento: fino a `>`, spazio o `/` (tag auto-chiuso).
        let tag_end = after.find(['>', ' ', '/'])?;
        let raw = &after[..tag_end];
        // Salta i tag di chiusura (`</d:name>`).
        if raw.starts_with('/') {
            search = start + 1;
            continue;
        }
        // Nome locale dopo l'eventuale prefisso di namespace.
        let local = match raw.rfind(':') {
            Some(i) => &raw[i + 1..],
            None => raw,
        };
        if local == name {
            let content_start = start + 1 + tag_end + 1; // subito dopo `>`
            let content = &xml[content_start..];
            let end = content.find('<')?;
            return Some(content[..end].to_string());
        }
        search = start + 1;
    }
    None
}

/// True if an XML tag with the given local name (any namespace prefix)
/// appears anywhere in `xml`.
fn contains_tag(xml: &str, name: &str) -> bool {
    let mut search = 0usize;
    while let Some(lt) = xml[search..].find('<') {
        let start = search + lt;
        let after = &xml[start + 1..];
        let Some(tag_end) = after.find(['>', ' ', '/']) else {
            return false;
        };
        let raw = &after[..tag_end];
        if raw.starts_with('/') {
            search = start + 1;
            continue;
        }
        let local = match raw.rfind(':') {
            Some(i) => &raw[i + 1..],
            None => raw,
        };
        if local == name {
            return true;
        }
        search = start + 1;
    }
    false
}

/// Extract `href`, `resourcetype`, `getcontentlength` from a PROPFIND
/// multistatus response. Paths are relative to `base_path`.
fn parse_multistatus(body: &[u8], base_path: &str) -> Result<Vec<Entry>, String> {
    let text = String::from_utf8_lossy(body);
    let mut entries = Vec::new();

    let rest = text.as_ref();
    let mut search_from = 0usize;
    loop {
        let open = find_response_open(rest, search_from);
        let Some((open_start, open_end)) = open else {
            break;
        };
        // Trova la chiusura `</d:response>` o `</response>` e ricorda la
        // lunghezza del tag per avanzare correttamente (13 vs 11 caratteri).
        let (close, close_len) = if let Some(i) = rest[open_end..].find("</d:response>") {
            (open_end + i, 13usize)
        } else if let Some(i) = rest[open_end..].find("</response>") {
            (open_end + i, 11usize)
        } else {
            break;
        };
        let block = &rest[open_start..close];
        search_from = close + close_len;

        let Some(href) = xml_tag(block, "href") else {
            continue;
        };
        let href = percent_decode(href.trim());

        let is_dir = contains_tag(block, "collection");
        let size = xml_tag(block, "getcontentlength")
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);
        let modified_secs =
            xml_tag(block, "getlastmodified").and_then(|s| super::dates::parse_rfc1123(s.trim()));

        let rel = relative_to_base(&href, base_path);
        let name = rel
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("/")
            .to_string();

        // Skip the collection itself (path == ""), because it represents
        // the folder we already requested.
        if rel.is_empty() {
            continue;
        }

        entries.push(Entry {
            name,
            path: if rel.starts_with('/') {
                rel.clone()
            } else {
                format!("/{rel}")
            },
            is_dir,
            size,
            modified_secs,
        });
    }

    Ok(entries)
}

fn find_response_open(text: &str, from: usize) -> Option<(usize, usize)> {
    for pat in ["<d:response>", "<response>", "<d:response ", "<response "] {
        if let Some(pos) = text[from..].find(pat) {
            let start = from + pos;
            let end = text[start..].find('>').map(|i| start + i + 1)?;
            return Some((start, end));
        }
    }
    None
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &s[i + 1..i + 3];
            if let Ok(v) = u8::from_str_radix(hex, 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Given an absolute href and `base_path`, return the relative path
/// (without `base_path`). Returns an empty string if it matches the base.
fn relative_to_base(href: &str, base_path: &str) -> String {
    // Alcuni server restituiscono href assoluti (`https://host/...`):
    // rimuovi scheme e host prima del confronto con la base.
    let href = strip_scheme_host(href);
    let href = href.trim_end_matches('/');
    let base = base_path.trim_end_matches('/');
    if base.is_empty() {
        return href.to_string();
    }
    if href == base {
        return String::new();
    }
    if let Some(rest) = href.strip_prefix(&format!("{base}/")) {
        rest.to_string()
    } else if let Some(rest) = href.strip_prefix(base) {
        rest.to_string()
    } else {
        href.to_string()
    }
}

/// Rimuove `http://`/`https://` e l'host da un href assoluto, lasciando
/// il path (es. `https://h/a/b` → `/a/b`). Invariato se già relativo.
fn strip_scheme_host(href: &str) -> &str {
    let rest = href
        .strip_prefix("https://")
        .or_else(|| href.strip_prefix("http://"));
    match rest {
        Some(r) => match r.find('/') {
            Some(i) => &r[i..],
            None => "/",
        },
        None => href,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_https_with_default_port() {
        let u = parse_url("https://cloud.example.com/remote.php/dav/files/user/").unwrap();
        assert!(u.secure);
        assert_eq!(u.host, "cloud.example.com");
        assert_eq!(u.port, 443);
        assert_eq!(u.base_path, "/remote.php/dav/files/user/");
    }

    #[test]
    fn parse_http_with_custom_port() {
        let u = parse_url("http://localhost:8080/dav/").unwrap();
        assert!(!u.secure);
        assert_eq!(u.host, "localhost");
        assert_eq!(u.port, 8080);
        assert_eq!(u.base_path, "/dav/");
    }

    #[test]
    fn url_encode_keeps_slashes() {
        assert_eq!(url_encode_path("/a b/c.txt"), "/a%20b/c.txt");
    }

    #[test]
    fn percent_decode_roundtrip() {
        assert_eq!(percent_decode("/a%20b/c.txt"), "/a b/c.txt");
    }
}
