//! Low-level HTTP(S) request helper (WinHTTP on Windows).
//!
//! Used by the Dropbox and WebDAV clients. Kept separate from
//! [`super::http`] (the `wxHTTP`-style client) because these clients
//! need full control over the method, TCP port, TLS, request headers
//! and the raw HTTP status code.

/// Perform a single HTTP(S) request and return `(status_code, body)`.
///
/// `host` is a bare host name (no scheme / port), `port` the TCP port,
/// `secure` enables TLS, `path` the URL path beginning with `/`,
/// `headers` are `name: value` pairs, and `body` the request payload.
///
/// On transport failure returns `Err(message)`; HTTP-level errors (4xx /
/// 5xx) are *not* transport failures — the caller inspects `status_code`
/// and the error body.
pub fn request(
    method: &str,
    host: &str,
    port: u16,
    secure: bool,
    path: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<(u32, Vec<u8>), String> {
    #[cfg(target_os = "windows")]
    {
        request_winhttp(method, host, port, secure, path, headers, body)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (method, host, port, secure, path, headers, body);
        Err("HTTP(S) requests require the Windows WinHTTP backend".into())
    }
}

/// Generate the `Authorization: Basic …` header for HTTP Basic auth.
pub fn basic_auth_header(username: &str, password: &str) -> (String, String) {
    (
        "Authorization".to_string(),
        format!("Basic {}", base64_encode(&format!("{username}:{password}"))),
    )
}

/// Encode `s` in Base64 (RFC 4648, standard alphabet with padding).
fn base64_encode(s: &str) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = s.as_bytes();
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
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let b0 = bytes[i] as u32;
        let b1 = bytes[i + 1] as u32;
        out.push(CHARS[(b0 >> 2) as usize] as char);
        out.push(CHARS[(((b0 & 0b11) << 4) | (b1 >> 4)) as usize] as char);
        out.push(CHARS[((b1 & 0b1111) << 2) as usize] as char);
        out.push('=');
    }
    out
}

#[cfg(target_os = "windows")]
fn request_winhttp(
    method: &str,
    host: &str,
    port: u16,
    secure: bool,
    path: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<(u32, Vec<u8>), String> {
    use windows_sys::Win32::Networking::WinHttp::{
        WinHttpAddRequestHeaders, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen,
        WinHttpOpenRequest, WinHttpQueryDataAvailable, WinHttpQueryHeaders, WinHttpReadData,
        WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetTimeouts,
    };

    const ACCESS_TYPE_DEFAULT_PROXY: u32 = 0;
    const FLAG_SECURE: u32 = 0x0080_0000;
    const QUERY_STATUS_CODE: u32 = 19;
    const QUERY_FLAG_NUMBER: u32 = 0x2000_0000;
    const ADDREQ_FLAG_ADD: u32 = 0x2000_0000;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    // SAFETY: all WinHTTP handles are checked for null after creation
    // and released exactly once on every exit path below.
    unsafe {
        let session = WinHttpOpen(
            wide(concat!("ru_wx/", env!("CARGO_PKG_VERSION"))).as_ptr(),
            ACCESS_TYPE_DEFAULT_PROXY,
            std::ptr::null(),
            std::ptr::null(),
            0,
        );
        if session.is_null() {
            return Err("WinHttpOpen failed".into());
        }

        let connect = WinHttpConnect(session, wide(host).as_ptr(), port, 0);
        if connect.is_null() {
            WinHttpCloseHandle(session);
            return Err("WinHttpConnect failed".into());
        }

        let flags = if secure { FLAG_SECURE } else { 0 };
        let request = WinHttpOpenRequest(
            connect,
            wide(method).as_ptr(),
            wide(path).as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            flags,
        );
        if request.is_null() {
            WinHttpCloseHandle(connect);
            WinHttpCloseHandle(session);
            return Err("WinHttpOpenRequest failed".into());
        }

        // 15s resolve/connect, 60s send/receive (uploads/downloads can
        // be large).
        let _ = WinHttpSetTimeouts(request, 15000, 15000, 60000, 60000);

        for (name, value) in headers {
            let hdr = format!("{name}: {value}\r\n");
            let hw = wide(&hdr);
            let len = (hw.len() as u32).saturating_sub(1); // exclude NUL
            let _ = WinHttpAddRequestHeaders(request, hw.as_ptr(), len, ADDREQ_FLAG_ADD);
        }

        let body_ptr: *const core::ffi::c_void = if body.is_empty() {
            std::ptr::null()
        } else {
            body.as_ptr() as *const core::ffi::c_void
        };
        let send_ok = WinHttpSendRequest(
            request,
            std::ptr::null(),
            0,
            body_ptr,
            body.len() as u32,
            body.len() as u32,
            0,
        );
        if send_ok == 0 {
            WinHttpCloseHandle(request);
            WinHttpCloseHandle(connect);
            WinHttpCloseHandle(session);
            return Err("WinHttpSendRequest failed".into());
        }

        if WinHttpReceiveResponse(request, std::ptr::null_mut()) == 0 {
            WinHttpCloseHandle(request);
            WinHttpCloseHandle(connect);
            WinHttpCloseHandle(session);
            return Err("WinHttpReceiveResponse failed".into());
        }

        // Read the HTTP status code as a DWORD.
        let mut status: u32 = 0;
        let mut status_len: u32 = std::mem::size_of::<u32>() as u32;
        let _ = WinHttpQueryHeaders(
            request,
            QUERY_STATUS_CODE | QUERY_FLAG_NUMBER,
            std::ptr::null(),
            &mut status as *mut u32 as *mut core::ffi::c_void,
            &mut status_len,
            std::ptr::null_mut(),
        );

        // Accumulate the response body.
        let mut out: Vec<u8> = Vec::new();
        loop {
            let mut available: u32 = 0;
            if WinHttpQueryDataAvailable(request, &mut available) == 0 || available == 0 {
                break;
            }
            let mut buf = vec![0u8; available as usize];
            let mut read: u32 = 0;
            if WinHttpReadData(
                request,
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                available,
                &mut read,
            ) == 0
                || read == 0
            {
                break;
            }
            buf.truncate(read as usize);
            out.extend_from_slice(&buf);
        }

        WinHttpCloseHandle(request);
        WinHttpCloseHandle(connect);
        WinHttpCloseHandle(session);

        Ok((status, out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_auth_header_is_correct() {
        assert_eq!(
            basic_auth_header("user", "pass"),
            (
                "Authorization".to_string(),
                "Basic dXNlcjpwYXNz".to_string()
            )
        );
    }
}
