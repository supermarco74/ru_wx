//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Socket events (`wxSocketEvent`).

/// Socket notification (`wxSocketEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketEventKind {
    Input,
    Output,
    Connection,
    Lost,
}

/// Event delivered by [`crate::Socket`] (`wxSocketEvent`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketEvent {
    pub kind: SocketEventKind,
    pub bytes_available: usize,
}

impl SocketEvent {
    pub const fn new(kind: SocketEventKind, bytes_available: usize) -> Self {
        Self {
            kind,
            bytes_available,
        }
    }
}
