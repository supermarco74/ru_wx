//! Debug-only tooling. Attualmente contiene l'**inspector** (Livello 1
//! del piano `PIANO_DEBUG_INSPECTOR.md`): un server HTTP locale che
//! espone lo stato della GUI (albero dei componenti, metadati finestra)
//! e, in prospettiva, azioni e screenshot.
//!
//! Vincolo di compilazione: in questo modulo **non** devono comparire
//! `objc2` né `windows-sys`. L'inspector parla solo con le API pubbliche
//! di `ru_wx` (Livello 2) e, tramite esse, con i backend (Livello 3).

pub mod inspector;
