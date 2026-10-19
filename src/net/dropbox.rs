//! Dropbox API v2 client (list / download / upload / create folder /
//! delete / move) built on `super::http_request`.
//!
//! Uses a personal access token (`Authorization: Bearer …`). No external
//! HTTP crate is required: everything goes through WinHTTP.

use serde_json::Value;

use super::http_request;

const API_HOST: &str = "api.dropboxapi.com";
const CONTENT_HOST: &str = "content.dropboxapi.com";

/// A single entry returned by `list_folder`.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    /// Full Dropbox path (e.g. `/folder/file.txt`).
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    /// Last modification time (seconds since the Unix epoch), from
    /// Dropbox's `server_modified` field. `None` when absent/unparsable.
    pub modified_secs: Option<u64>,
}

/// Convenience wrapper for an HTTPS request to a Dropbox host.
fn https_request(
    method: &str,
    host: &str,
    path: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<(u32, Vec<u8>), String> {
    http_request::request(method, host, 443, true, path, headers, body)
}

/// Serialize `s` as a JSON string (with quoting and escaping).
fn jq(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

/// Extract a readable error message from a Dropbox JSON error body.
fn parse_error(body: &[u8]) -> String {
    let fallback = String::from_utf8_lossy(body).trim().to_string();
    match serde_json::from_slice::<Value>(body) {
        Ok(v) => v["error_summary"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn auth_header(token: &str) -> (String, String) {
    ("Authorization".to_string(), format!("Bearer {token}"))
}

/// List the contents of a Dropbox folder (with pagination).
pub fn list_folder(token: &str, path: &str) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    // La prima pagina usa `list_folder`; le successive (`has_more`) usano
    // `list_folder/continue` con il cursore restituito dalla pagina prima.
    let mut endpoint = "/2/files/list_folder";
    let mut body = format!(
        r#"{{"path":{},"recursive":false,"include_media_info":false,"include_deleted":false}}"#,
        jq(path)
    );

    loop {
        let (status, resp) = https_request(
            "POST",
            API_HOST,
            endpoint,
            &[
                auth_header(token),
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body.as_bytes(),
        )?;
        if status != 200 {
            return Err(parse_error(&resp));
        }
        let v: Value = serde_json::from_slice(&resp).map_err(|e| e.to_string())?;
        if let Some(list) = v["entries"].as_array() {
            for e in list {
                let modified_secs = e["server_modified"]
                    .as_str()
                    .and_then(super::dates::parse_iso8601_utc);
                entries.push(Entry {
                    name: e["name"].as_str().unwrap_or("").to_string(),
                    path: e["path_display"].as_str().unwrap_or("").to_string(),
                    is_dir: e[".tag"].as_str() == Some("folder"),
                    size: e["size"].as_u64().unwrap_or(0),
                    modified_secs,
                });
            }
        }
        let has_more = v["has_more"].as_bool().unwrap_or(false);
        if !has_more {
            break;
        }
        let cursor = v["cursor"]
            .as_str()
            .ok_or_else(|| "missing pagination cursor".to_string())?;
        endpoint = "/2/files/list_folder/continue";
        body = format!(r#"{{"cursor":{}}}"#, jq(cursor));
    }

    Ok(entries)
}

/// Download a Dropbox file and return its bytes.
pub fn download(token: &str, path: &str) -> Result<Vec<u8>, String> {
    let arg = format!(r#"{{"path":{}}}"#, jq(path));
    let (status, resp) = https_request(
        "POST",
        CONTENT_HOST,
        "/2/files/download",
        &[auth_header(token), ("Dropbox-API-Arg".to_string(), arg)],
        &[],
    )?;
    if status != 200 {
        return Err(parse_error(&resp));
    }
    Ok(resp)
}

/// Upload a local file to Dropbox (destination = full path).
pub fn upload(token: &str, local_src: &std::path::Path, dropbox_dest: &str) -> Result<(), String> {
    let data = std::fs::read(local_src).map_err(|e| e.to_string())?;
    let arg = format!(
        r#"{{"path":{},"mode":"add","autorename":true,"mute":false}}"#,
        jq(dropbox_dest)
    );
    let (status, resp) = https_request(
        "POST",
        CONTENT_HOST,
        "/2/files/upload",
        &[
            auth_header(token),
            ("Dropbox-API-Arg".to_string(), arg),
            (
                "Content-Type".to_string(),
                "application/octet-stream".to_string(),
            ),
        ],
        &data,
    )?;
    if status != 200 {
        return Err(parse_error(&resp));
    }
    Ok(())
}

/// Create a Dropbox folder (tolerates an already-existing folder: 409).
pub fn create_folder(token: &str, path: &str) -> Result<(), String> {
    let body = format!(r#"{{"path":{},"autorename":false}}"#, jq(path));
    let (status, resp) = https_request(
        "POST",
        API_HOST,
        "/2/files/create_folder_v2",
        &[
            auth_header(token),
            ("Content-Type".to_string(), "application/json".to_string()),
        ],
        body.as_bytes(),
    )?;
    // 200 = created, 409 = already exists (not an error for us).
    if status != 200 && status != 409 {
        return Err(parse_error(&resp));
    }
    Ok(())
}

/// Permanently delete a Dropbox file or folder.
pub fn delete(token: &str, path: &str) -> Result<(), String> {
    let body = format!(r#"{{"path":{}}}"#, jq(path));
    let (status, resp) = https_request(
        "POST",
        API_HOST,
        "/2/files/delete_v2",
        &[
            auth_header(token),
            ("Content-Type".to_string(), "application/json".to_string()),
        ],
        body.as_bytes(),
    )?;
    if status != 200 {
        return Err(parse_error(&resp));
    }
    Ok(())
}

/// Move / rename a Dropbox file or folder (`from_path` → `to_path`).
pub fn move_path(token: &str, from_path: &str, to_path: &str) -> Result<(), String> {
    let body = format!(
        r#"{{"from_path":{},"to_path":{}}}"#,
        jq(from_path),
        jq(to_path)
    );
    let (status, resp) = https_request(
        "POST",
        API_HOST,
        "/2/files/move_v2",
        &[
            auth_header(token),
            ("Content-Type".to_string(), "application/json".to_string()),
        ],
        body.as_bytes(),
    )?;
    if status != 200 {
        return Err(parse_error(&resp));
    }
    Ok(())
}

// ── OAuth2 (Authorization Code + PKCE) ─────────────────────────────
//
// Flusso "public client" consigliato per app desktop: niente App secret,
// solo App key (`client_id`), `code_verifier`/`code_challenge` S256 e
// `token_access_type=offline` per ottenere un refresh token a lunga
// durata (l'access token `sl.` scade dopo 4 ore).

const OAUTH_HOST: &str = "api.dropboxapi.com";

/// Token restituiti dall'endpoint `/oauth2/token`.
#[derive(Debug, Clone)]
pub struct OAuthTokens {
    pub access_token: String,
    /// Presente solo nello scambio del codice di autorizzazione; la
    /// chiamata di refresh non restituisce un nuovo refresh token.
    pub refresh_token: Option<String>,
    pub expires_in: u64,
    pub token_type: String,
    pub account_id: String,
}

/// Base64 URL-safe senza padding (RFC 4648 §5), alfabeto `-`/`_`.
pub fn base64url_no_pad(bytes: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = bytes[i + 1] as u32;
        let b2 = bytes[i + 2] as u32;
        out.push(CHARS[(b0 >> 2) as usize] as char);
        out.push(CHARS[(((b0 & 0b11) << 4) | (b1 >> 4)) as usize] as char);
        out.push(CHARS[(((b1 & 0b1111) << 2) | (b2 >> 6)) as usize] as char);
        out.push(CHARS[(b2 & 0b111111) as usize] as char);
        i += 3;
    }
    let rem = bytes.len() - i;
    if rem == 1 {
        let b0 = bytes[i] as u32;
        out.push(CHARS[(b0 >> 2) as usize] as char);
        out.push(CHARS[((b0 & 0b11) << 4) as usize] as char);
    } else if rem == 2 {
        let b0 = bytes[i] as u32;
        let b1 = bytes[i + 1] as u32;
        out.push(CHARS[(b0 >> 2) as usize] as char);
        out.push(CHARS[(((b0 & 0b11) << 4) | (b1 >> 4)) as usize] as char);
        out.push(CHARS[((b1 & 0b1111) << 2) as usize] as char);
    }
    out
}

/// `code_challenge` S256: Base64url(SHA-256(code_verifier)) senza
/// padding, come richiesto da RFC 7636 §4.2.
pub fn code_challenge_s256(verifier: &str) -> String {
    base64url_no_pad(&sha256(verifier.as_bytes()))
}

/// Costruisce l'URL di autorizzazione da aprire nel browser.
pub fn oauth_authorize_url(
    app_key: &str,
    redirect_uri: &str,
    code_challenge: &str,
    state: &str,
) -> String {
    format!(
        "https://www.dropbox.com/oauth2/authorize?client_id={}&token_access_type=offline&response_type=code&redirect_uri={}&code_challenge={}&code_challenge_method=S256&state={}",
        urlencode(app_key),
        urlencode(redirect_uri),
        urlencode(code_challenge),
        urlencode(state),
    )
}

/// Scambia il codice di autorizzazione con access token + refresh token.
pub fn oauth_exchange_code(
    app_key: &str,
    redirect_uri: &str,
    code: &str,
    code_verifier: &str,
) -> Result<OAuthTokens, String> {
    let body = format!(
        "code={}&grant_type=authorization_code&client_id={}&code_verifier={}&redirect_uri={}",
        urlencode(code),
        urlencode(app_key),
        urlencode(code_verifier),
        urlencode(redirect_uri),
    );
    oauth_token_request(&body)
}

/// Ottiene un nuovo access token a partire dal refresh token.
pub fn oauth_refresh(app_key: &str, refresh_token: &str) -> Result<OAuthTokens, String> {
    let body = format!(
        "grant_type=refresh_token&refresh_token={}&client_id={}",
        urlencode(refresh_token),
        urlencode(app_key),
    );
    oauth_token_request(&body)
}

fn oauth_token_request(body: &str) -> Result<OAuthTokens, String> {
    let (status, resp) = https_request(
        "POST",
        OAUTH_HOST,
        "/oauth2/token",
        &[(
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        )],
        body.as_bytes(),
    )?;
    if status != 200 {
        return Err(parse_error(&resp));
    }
    let v: Value = serde_json::from_slice(&resp).map_err(|e| e.to_string())?;
    let access_token = v["access_token"].as_str().unwrap_or("").to_string();
    if access_token.is_empty() {
        return Err("token response senza access_token".to_string());
    }
    Ok(OAuthTokens {
        access_token,
        refresh_token: v["refresh_token"].as_str().map(|s| s.to_string()),
        expires_in: v["expires_in"].as_u64().unwrap_or(14400),
        token_type: v["token_type"].as_str().unwrap_or("bearer").to_string(),
        account_id: v["account_id"].as_str().unwrap_or("").to_string(),
    })
}

/// Percent-encoding `application/x-www-form-urlencoded` (RFC 3986 set).
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// SHA-256 (FIPS 180-4), implementazione self-contained senza dipendenze.
fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().enumerate().take(16) {
            *word = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hv = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hv
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hv = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hv);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{base64url_no_pad, code_challenge_s256, jq, parse_error, sha256};

    #[test]
    fn jq_quotes_and_escapes() {
        assert_eq!(jq("hello"), "\"hello\"");
        assert_eq!(jq("/a/b"), "\"/a/b\"");
    }

    #[test]
    fn parse_error_extracts_summary() {
        let body = br#"{"error_summary": "path/not_found/."}"#;
        assert_eq!(parse_error(body), "path/not_found/.");
    }

    #[test]
    fn parse_error_falls_back_to_raw_text() {
        assert_eq!(parse_error(b"not json at all"), "not json at all");
    }

    #[test]
    fn base64url_no_pad_matches_rfc4648() {
        assert_eq!(base64url_no_pad(b"f"), "Zg");
        assert_eq!(base64url_no_pad(b"fo"), "Zm8");
        assert_eq!(base64url_no_pad(b"foo"), "Zm9v");
    }

    #[test]
    fn sha256_matches_fips_vectors() {
        // SHA-256("") e SHA-256("abc") in Base64url senza padding.
        assert_eq!(
            base64url_no_pad(&sha256(b"")),
            "47DEQpj8HBSa-_TImW-5JCeuQeRkm5NMpJWZG3hSuFU"
        );
        assert_eq!(
            base64url_no_pad(&sha256(b"abc")),
            "ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0"
        );
    }

    #[test]
    fn pkce_challenge_matches_rfc7636() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            code_challenge_s256(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }
}
