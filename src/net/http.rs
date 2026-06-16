//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! HTTP client (`wxHTTP`).

use std::io;

/// HTTP session (`wxHTTP`).
#[derive(Debug, Default)]
pub struct HttpClient {
    base_url: String,
}

impl HttpClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_base_url(&mut self, url: &str) {
        self.base_url = url.to_string();
    }

    pub fn get(&self, path: &str) -> io::Result<Vec<u8>> {
        let url = resolve_url(&self.base_url, path);
        http_request("GET", &url, None)
    }

    pub fn post(&self, path: &str, body: &[u8]) -> io::Result<Vec<u8>> {
        let url = resolve_url(&self.base_url, path);
        http_request("POST", &url, Some(body))
    }
}

fn http_request(method: &str, url: &str, body: Option<&[u8]>) -> io::Result<Vec<u8>> {
    #[cfg(target_os = "windows")]
    {
        win_http_request(method, url, body)
    }
    #[cfg(not(target_os = "windows"))]
    {
        tcp_http_request(method, url, body)
    }
}

fn resolve_url(base: &str, path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") || base.is_empty() {
        path.to_string()
    } else {
        format!("{}{}", base.trim_end_matches('/'), path)
    }
}

#[cfg(target_os = "windows")]
fn win_http_request(method: &str, url: &str, body: Option<&[u8]>) -> io::Result<Vec<u8>> {
    use std::ffi::c_void;
    use std::ptr;

    use windows_sys::Win32::Networking::WinHttp::{
        WinHttpAddRequestHeaders, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen,
        WinHttpOpenRequest, WinHttpQueryDataAvailable, WinHttpReadData, WinHttpReceiveResponse,
        WinHttpSendRequest, WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, WINHTTP_FLAG_SECURE,
    };

    let (host, path, secure) = parse_http_url(url)?;
    let host_wide = utf16z(&host);
    let path_wide = utf16z(&path);
    let method_wide = utf16z(method);
    let user_agent = utf16z("ru_wx/0.6.4");

    // SAFETY: WinHTTP session/connect/request lifecycle.
    unsafe {
        let session = WinHttpOpen(
            user_agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
            ptr::null(),
            ptr::null(),
            0,
        );
        if session.is_null() {
            return Err(io::Error::other("WinHttpOpen failed"));
        }
        let connect = WinHttpConnect(session, host_wide.as_ptr(), if secure { 443 } else { 80 }, 0);
        if connect.is_null() {
            WinHttpCloseHandle(session);
            return Err(io::Error::other("WinHttpConnect failed"));
        }
        let request = WinHttpOpenRequest(
            connect,
            method_wide.as_ptr(),
            path_wide.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            if secure { WINHTTP_FLAG_SECURE } else { 0 },
        );
        if request.is_null() {
            WinHttpCloseHandle(connect);
            WinHttpCloseHandle(session);
            return Err(io::Error::other("WinHttpOpenRequest failed"));
        }
        if method == "POST" {
            let headers = utf16z("Content-Type: application/octet-stream\r\n");
            WinHttpAddRequestHeaders(
                request,
                headers.as_ptr(),
                (headers.len() as u32).saturating_sub(1),
                0x20000000,
            );
        }
        let send_ok = WinHttpSendRequest(
            request,
            ptr::null(),
            0,
            body.map(|b| b.as_ptr() as *mut c_void).unwrap_or(ptr::null_mut()),
            body.map(|b| b.len() as u32).unwrap_or(0),
            body.map(|b| b.len() as u32).unwrap_or(0),
            0,
        );
        if send_ok == 0 {
            WinHttpCloseHandle(request);
            WinHttpCloseHandle(connect);
            WinHttpCloseHandle(session);
            return Err(io::Error::other("WinHttpSendRequest failed"));
        }
        if WinHttpReceiveResponse(request, ptr::null_mut()) == 0 {
            WinHttpCloseHandle(request);
            WinHttpCloseHandle(connect);
            WinHttpCloseHandle(session);
            return Err(io::Error::other("WinHttpReceiveResponse failed"));
        }
        let mut out = Vec::new();
        loop {
            let mut available = 0u32;
            if WinHttpQueryDataAvailable(request, &mut available) == 0 {
                break;
            }
            if available == 0 {
                break;
            }
            let start = out.len();
            out.resize(start + available as usize, 0);
            let mut read = 0u32;
            if WinHttpReadData(
                request,
                out[start..].as_mut_ptr() as *mut c_void,
                available,
                &mut read,
            ) == 0
            {
                break;
            }
            out.truncate(start + read as usize);
            if read == 0 {
                break;
            }
        }
        WinHttpCloseHandle(request);
        WinHttpCloseHandle(connect);
        WinHttpCloseHandle(session);
        Ok(out)
    }
}

#[cfg(target_os = "windows")]
fn parse_http_url(url: &str) -> io::Result<(String, String, bool)> {
    parse_http_url_shared(url)
}

fn parse_http_url_shared(url: &str) -> io::Result<(String, String, bool)> {
    let (secure, rest) = if let Some(r) = url.strip_prefix("https://") {
        (true, r)
    } else if let Some(r) = url.strip_prefix("http://") {
        (false, r)
    } else {
        (false, url)
    };
    let (host, path) = match rest.split_once('/') {
        Some((h, p)) => (h.to_string(), format!("/{p}")),
        None => (rest.to_string(), "/".to_string()),
    };
    Ok((host, path, secure))
}

#[cfg(not(target_os = "windows"))]
fn parse_http_url(url: &str) -> io::Result<(String, String, bool)> {
    parse_http_url_shared(url)
}

#[cfg(not(target_os = "windows"))]
fn tcp_http_request(method: &str, url: &str, body: Option<&[u8]>) -> io::Result<Vec<u8>> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let (host, path, secure) = parse_http_url(url)?;
    if secure {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "HTTPS requires the Windows WinHTTP backend in this build",
        ));
    }
    let mut stream = TcpStream::connect(format!("{host}:80"))?;
    let request = match (method, body) {
        ("GET", _) => format!(
            "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
        ),
        ("POST", Some(payload)) => format!(
            "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            payload.len()
        ),
        ("POST", None) => format!(
            "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ),
        other => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("unsupported HTTP method: {}", other.0),
            ))
        }
    };
    stream.write_all(request.as_bytes())?;
    if method == "POST" {
        if let Some(payload) = body {
            stream.write_all(payload)?;
        }
    }
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    extract_http_body(&response)
}

fn extract_http_body(response: &[u8]) -> io::Result<Vec<u8>> {
    if let Some(pos) = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
    {
        return Ok(response[pos + 4..].to_vec());
    }
    if let Some(pos) = response.windows(2).position(|w| w == b"\n\n") {
        return Ok(response[pos + 2..].to_vec());
    }
    Ok(response.to_vec())
}

#[cfg(target_os = "windows")]
fn utf16z(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_url_joins_base() {
        assert_eq!(
            resolve_url("http://example.com", "/api"),
            "http://example.com/api"
        );
        assert_eq!(
            resolve_url("", "https://x.test/path"),
            "https://x.test/path"
        );
    }
}
