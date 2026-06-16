//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! FTP client (`wxFTP`).

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;

/// FTP session (`wxFTP`).
#[derive(Debug, Default)]
pub struct FtpClient {
    host: String,
    control: Option<TcpStream>,
    connected: bool,
    logged_in: bool,
}

impl FtpClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn connect(&mut self, host: &str, user: &str, password: &str) -> io::Result<()> {
        let mut stream = TcpStream::connect(format!("{host}:21"))?;
        read_reply(&mut stream)?;
        send_cmd(&mut stream, &format!("USER {user}"))?;
        let code = read_reply_code(&mut stream)?;
        if code == 331 {
            send_cmd(&mut stream, &format!("PASS {password}"))?;
            read_reply_code(&mut stream)?;
        }
        self.host = host.to_string();
        self.control = Some(stream);
        self.connected = true;
        self.logged_in = true;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        if let Some(mut stream) = self.control.take() {
            let _ = send_cmd(&mut stream, "QUIT");
            let _ = read_reply(&mut stream);
        }
        self.connected = false;
        self.logged_in = false;
    }

    pub fn is_connected(&self) -> bool {
        self.connected && self.logged_in && self.control.is_some()
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn get_file(&self, remote: &str) -> io::Result<Vec<u8>> {
        let control = self
            .control
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "FTP not connected"))?;

        let mut control = control.try_clone().map_err(io::Error::other)?;

        send_cmd(&mut control, "PASV")?;
        let pasv_line = read_reply_line(&mut control)?;
        let data_addr = parse_pasv(&pasv_line)?;

        send_cmd(&mut control, &format!("RETR {remote}"))?;
        let code = read_reply_code(&mut control)?;
        if code >= 400 {
            return Err(io::Error::other(format!("FTP RETR failed: {code}")));
        }

        let mut data = TcpStream::connect(data_addr)?;
        let mut buf = Vec::new();
        data.read_to_end(&mut buf)?;
        read_reply_code(&mut control)?;
        Ok(buf)
    }
}

fn send_cmd(stream: &mut TcpStream, cmd: &str) -> io::Result<()> {
    let line = format!("{cmd}\r\n");
    stream.write_all(line.as_bytes())
}

fn read_reply_line(stream: &mut TcpStream) -> io::Result<String> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(line)
}

fn read_reply_code(stream: &mut TcpStream) -> io::Result<u16> {
    let line = read_reply_line(stream)?;
    parse_reply_code(&line)
}

fn read_reply(stream: &mut TcpStream) -> io::Result<()> {
    let _ = read_reply_code(stream)?;
    Ok(())
}

fn parse_reply_code(line: &str) -> io::Result<u16> {
    line.get(0..3)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("bad FTP reply: {line}")))
}

fn parse_pasv(line: &str) -> io::Result<String> {
    let start = line
        .find('(')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "PASV missing '('"))?
        + 1;
    let end = line
        .find(')')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "PASV missing ')'"))?;
    let nums: Vec<u8> = line[start..end]
        .split(',')
        .map(|s| s.trim().parse())
        .collect::<Result<_, _>>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad PASV numbers"))?;
    if nums.len() != 6 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "PASV expected 6 numbers",
        ));
    }
    let port = u16::from(nums[4]) * 256 + u16::from(nums[5]);
    Ok(format!("{}.{}.{}.{}:{port}", nums[0], nums[1], nums[2], nums[3]))
}
