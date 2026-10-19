//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Demo: browser cloud minimale costruito sopra i moduli pubblici
//! `ru_wx::net::dropbox` e `ru_wx::net::webdav`.
//!
//! Mostra come usare i client cloud esposti dalla libreria (gli stessi
//! usati da `rust_explorer`) da un programma qualsiasi:
//!
//! - configurazione dei parametri (token Dropbox + URL/utente/password
//!   WebDAV) tramite una dialog "Impostazioni" salvata in un file JSON
//!   accanto all'eseguibile;
//! - navigazione cartelle, creazione cartella, upload, download,
//!   rinomina ed eliminazione;
//! - operazioni di rete su thread worker, con risultato postato al
//!   thread UI tramite `Frame::post_raw_message_ptr` (l'UI non si
//!   blocca mai).
//!
//! Esegui con:
//! ```bash
//! cargo run --example cloud_demo
//! ```

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use ru_wx::net::{dropbox, webdav};
use ru_wx::{
    message_box, App, BoxSizer, Button, Dialog, FileDialog, FileDialogStyle, Frame, ListCtrl,
    ListCtrlStyle, Menu, MenuBar, MessageBoxIcon, MessageBoxStyle, StaticText, TextCtrl,
    TextEntryDialog,
};

// ─── Messaggi custom (worker → UI) ─────────────────────────────────────────
/// Risultato di un listing (popola la lista).
const MSG_LIST: u32 = 0x0501;
/// Esito di un'operazione singola (mostra una message box).
const MSG_OP: u32 = 0x0502;

// ─── Provider ──────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq)]
enum Provider {
    Dropbox,
    Webdav,
}

// ─── Configurazione (persistita in JSON) ───────────────────────────────────
#[derive(Clone)]
struct Config {
    provider: String,
    dropbox_token: String,
    webdav_url: String,
    webdav_username: String,
    webdav_password: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            provider: "dropbox".to_string(),
            dropbox_token: String::new(),
            webdav_url: String::new(),
            webdav_username: String::new(),
            webdav_password: String::new(),
        }
    }
}

/// Percorso del file di configurazione, accanto all'eseguibile.
fn config_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cloud_demo_config.json")
}

fn load_config() -> Config {
    let Ok(bytes) = std::fs::read(config_path()) else {
        return Config::default();
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Config::default();
    };
    let s = |key: &str| -> String {
        v.get(key)
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string()
    };
    Config {
        provider: if s("provider") == "webdav" {
            "webdav".to_string()
        } else {
            "dropbox".to_string()
        },
        dropbox_token: s("dropbox_token"),
        webdav_url: s("webdav_url"),
        webdav_username: s("webdav_username"),
        webdav_password: s("webdav_password"),
    }
}

fn save_config(cfg: &Config) {
    let value = serde_json::json!({
        "provider": cfg.provider,
        "dropbox_token": cfg.dropbox_token,
        "webdav_url": cfg.webdav_url,
        "webdav_username": cfg.webdav_username,
        "webdav_password": cfg.webdav_password,
    });
    if let Ok(json) = serde_json::to_vec_pretty(&value) {
        let _ = std::fs::write(config_path(), json);
    }
}

fn provider_from_str(s: &str) -> Provider {
    if s == "webdav" {
        Provider::Webdav
    } else {
        Provider::Dropbox
    }
}

// ─── Elemento di lista unificato ────────────────────────────────────────────
#[derive(Clone)]
struct Item {
    name: String,
    /// Percorso remoto: intero per Dropbox (`/a/b`), relativo alla base
    /// per WebDAV (`` per la root, `/a/b` per le sottocartelle).
    path: String,
    is_dir: bool,
    size: u64,
}

// ─── Stato UI condiviso ────────────────────────────────────────────────────
#[derive(Clone)]
struct UiState {
    provider: Provider,
    current_path: String,
}

// ─── Payload postati dai worker ────────────────────────────────────────────
struct ListPayload {
    provider: Provider,
    path: String,
    items: Vec<Item>,
    error: Option<String>,
}

struct OpPayload {
    title: String,
    message: String,
}

// ─── Helpers sui path ──────────────────────────────────────────────────────
fn dropbox_parent(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "/" {
        return None;
    }
    match trimmed.rfind('/') {
        Some(0) => Some("/".to_string()),
        Some(i) => Some(trimmed[..i].to_string()),
        None => Some("/".to_string()),
    }
}

fn webdav_parent(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.rfind('/') {
        Some(0) => Some(String::new()),
        Some(i) => Some(trimmed[..i].to_string()),
        None => Some(String::new()),
    }
}

fn parent_path(provider: Provider, path: &str) -> Option<String> {
    match provider {
        Provider::Dropbox => dropbox_parent(path),
        Provider::Webdav => webdav_parent(path),
    }
}

fn dropbox_join(dir: &str, name: &str) -> String {
    if dir.is_empty() || dir == "/" {
        format!("/{name}")
    } else {
        format!("{}/{}", dir.trim_end_matches('/'), name)
    }
}

fn webdav_join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        format!("/{name}")
    } else {
        format!("{}/{}", dir.trim_end_matches('/'), name)
    }
}

fn join_path(provider: Provider, dir: &str, name: &str) -> String {
    match provider {
        Provider::Dropbox => dropbox_join(dir, name),
        Provider::Webdav => webdav_join(dir, name),
    }
}

fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

// ─── Operazioni remote (eseguite sui worker) ───────────────────────────────
fn list_remote(cfg: &Config, provider: Provider, path: &str) -> Result<Vec<Item>, String> {
    match provider {
        Provider::Dropbox => {
            let entries = dropbox::list_folder(&cfg.dropbox_token, path)?;
            Ok(entries
                .into_iter()
                .map(|e| Item {
                    name: e.name,
                    path: e.path,
                    is_dir: e.is_dir,
                    size: e.size,
                })
                .collect())
        }
        Provider::Webdav => {
            let client =
                webdav::Client::new(&cfg.webdav_url, &cfg.webdav_username, &cfg.webdav_password)?;
            let entries = client.list(path)?;
            Ok(entries
                .into_iter()
                .map(|e| Item {
                    name: e.name,
                    path: e.path,
                    is_dir: e.is_dir,
                    size: e.size,
                })
                .collect())
        }
    }
}

fn create_folder_remote(cfg: &Config, provider: Provider, path: &str) -> Result<(), String> {
    match provider {
        Provider::Dropbox => dropbox::create_folder(&cfg.dropbox_token, path),
        Provider::Webdav => {
            let client =
                webdav::Client::new(&cfg.webdav_url, &cfg.webdav_username, &cfg.webdav_password)?;
            client.mkcol(path)
        }
    }
}

fn delete_remote(cfg: &Config, provider: Provider, path: &str) -> Result<(), String> {
    match provider {
        Provider::Dropbox => dropbox::delete(&cfg.dropbox_token, path),
        Provider::Webdav => {
            let client =
                webdav::Client::new(&cfg.webdav_url, &cfg.webdav_username, &cfg.webdav_password)?;
            client.delete(path)
        }
    }
}

fn rename_remote(cfg: &Config, provider: Provider, from: &str, to: &str) -> Result<(), String> {
    match provider {
        Provider::Dropbox => dropbox::move_path(&cfg.dropbox_token, from, to),
        Provider::Webdav => {
            let client =
                webdav::Client::new(&cfg.webdav_url, &cfg.webdav_username, &cfg.webdav_password)?;
            client.mv(from, to)
        }
    }
}

fn upload_remote(
    cfg: &Config,
    provider: Provider,
    local_src: &Path,
    dest: &str,
) -> Result<(), String> {
    match provider {
        Provider::Dropbox => dropbox::upload(&cfg.dropbox_token, local_src, dest),
        Provider::Webdav => {
            let data = std::fs::read(local_src).map_err(|e| e.to_string())?;
            let client =
                webdav::Client::new(&cfg.webdav_url, &cfg.webdav_username, &cfg.webdav_password)?;
            client.put(dest, &data)
        }
    }
}

fn download_remote(cfg: &Config, provider: Provider, path: &str) -> Result<Vec<u8>, String> {
    match provider {
        Provider::Dropbox => dropbox::download(&cfg.dropbox_token, path),
        Provider::Webdav => {
            let client =
                webdav::Client::new(&cfg.webdav_url, &cfg.webdav_username, &cfg.webdav_password)?;
            client.get(path)
        }
    }
}

// ─── Post asincroni (worker → UI) ──────────────────────────────────────────
/// Esegue `op`, mostra l'errore se fallisce, quindi ri-legge la cartella.
fn spawn_op_then_list(
    hwnd: isize,
    cfg: Config,
    provider: Provider,
    path: String,
    title: String,
    op: impl FnOnce(&Config) -> Result<(), String> + Send + 'static,
) {
    std::thread::spawn(move || {
        if let Err(e) = op(&cfg) {
            let payload = Box::new(OpPayload {
                title,
                message: format!("Operazione fallita:\n- {e}"),
            });
            Frame::post_raw_message_ptr(
                hwnd,
                MSG_OP,
                Box::into_raw(payload) as *mut std::ffi::c_void,
            );
        }
        post_list(hwnd, &cfg, provider, &path);
    });
}

/// Esegue il listing e posta il risultato al thread UI.
fn post_list(hwnd: isize, cfg: &Config, provider: Provider, path: &str) {
    let (items, error) = match list_remote(cfg, provider, path) {
        Ok(items) => (items, None),
        Err(e) => (Vec::new(), Some(e)),
    };
    let payload = Box::new(ListPayload {
        provider,
        path: path.to_string(),
        items,
        error,
    });
    Frame::post_raw_message_ptr(
        hwnd,
        MSG_LIST,
        Box::into_raw(payload) as *mut std::ffi::c_void,
    );
}

/// Avvia un semplice refresh della cartella corrente.
fn spawn_list(hwnd: isize, cfg: Config, provider: Provider, path: String) {
    std::thread::spawn(move || post_list(hwnd, &cfg, provider, &path));
}

// ─── UI helpers ────────────────────────────────────────────────────────────
/// Popola la `ListCtrl` con gli elementi correnti.
fn populate_list(list: &ListCtrl, items: &[Item]) {
    list.delete_all_items();
    for (idx, item) in items.iter().enumerate() {
        list.insert_item(idx, &item.name);
        list.set_item_text(idx, 1, if item.is_dir { "Cartella" } else { "File" });
        list.set_item_text(idx, 2, &human_size(item.size));
    }
}

/// Restituisce l'elemento selezionato (dal cache, allineato alle righe).
fn selected_item(list: &ListCtrl, cache: &Rc<RefCell<Vec<Item>>>) -> Option<Item> {
    let sel = list.get_selected_items();
    let idx = *sel.first()?;
    cache.borrow().get(idx).cloned()
}

/// Esegue un refresh usando lo stato condiviso corrente.
fn do_refresh(cfg: &Rc<RefCell<Config>>, state: &Rc<RefCell<UiState>>, hwnd: isize) {
    let (cfg_snap, provider, path) = {
        let c = cfg.borrow();
        let s = state.borrow();
        (c.clone(), s.provider, s.current_path.clone())
    };
    spawn_list(hwnd, cfg_snap, provider, path);
}

/// Mostra la dialog di configurazione e restituisce la config aggiornata.
fn show_settings(frame: &Frame, cfg: &Config) -> Config {
    let dlg = Dialog::new(frame, "Configurazione cloud", 480, 300);

    let lbl_dbx = StaticText::new(frame, "Dropbox — access token:");
    lbl_dbx.as_widget_ref().borrow_mut().set_position(20, 20);
    dlg.add_widget(lbl_dbx.as_widget_ref());

    let dbx = TextCtrl::new(frame, &cfg.dropbox_token);
    dbx.as_widget_ref().borrow_mut().set_position(20, 45);
    dbx.as_widget_ref().borrow_mut().set_size(440, 24);
    dlg.add_widget(dbx.as_widget_ref());

    let lbl_url = StaticText::new(
        frame,
        "WebDAV — URL (es. https://host/remote.php/dav/files/user/):",
    );
    lbl_url.as_widget_ref().borrow_mut().set_position(20, 80);
    dlg.add_widget(lbl_url.as_widget_ref());

    let url = TextCtrl::new(frame, &cfg.webdav_url);
    url.as_widget_ref().borrow_mut().set_position(20, 105);
    url.as_widget_ref().borrow_mut().set_size(440, 24);
    dlg.add_widget(url.as_widget_ref());

    let lbl_user = StaticText::new(frame, "WebDAV — utente:");
    lbl_user.as_widget_ref().borrow_mut().set_position(20, 140);
    dlg.add_widget(lbl_user.as_widget_ref());

    let user = TextCtrl::new(frame, &cfg.webdav_username);
    user.as_widget_ref().borrow_mut().set_position(20, 165);
    user.as_widget_ref().borrow_mut().set_size(440, 24);
    dlg.add_widget(user.as_widget_ref());

    let lbl_pass = StaticText::new(frame, "WebDAV — password:");
    lbl_pass.as_widget_ref().borrow_mut().set_position(20, 200);
    dlg.add_widget(lbl_pass.as_widget_ref());

    let pass = TextCtrl::password(frame, &cfg.webdav_password);
    pass.as_widget_ref().borrow_mut().set_position(20, 225);
    pass.as_widget_ref().borrow_mut().set_size(440, 24);
    dlg.add_widget(pass.as_widget_ref());

    let (id_ok, _) = dlg.create_button("Salva");
    let (id_cancel, _) = dlg.create_button("Annulla");

    let result = Rc::new(RefCell::new(cfg.clone()));

    let dbx_for_ok = dbx.clone();
    let url_for_ok = url.clone();
    let user_for_ok = user.clone();
    let pass_for_ok = pass.clone();
    let result_for_ok = Rc::clone(&result);
    let dlg_for_ok = dlg.clone();
    dlg.register_command_handler(
        id_ok,
        Box::new(move || {
            let mut r = result_for_ok.borrow_mut();
            r.dropbox_token = dbx_for_ok.get_value().trim().to_string();
            r.webdav_url = url_for_ok.get_value().trim().to_string();
            r.webdav_username = user_for_ok.get_value().trim().to_string();
            r.webdav_password = pass_for_ok.get_value();
            save_config(&r);
            dlg_for_ok.end_modal(1);
        }),
    );

    let dlg_for_cancel = dlg.clone();
    dlg.register_command_handler(
        id_cancel,
        Box::new(move || {
            dlg_for_cancel.end_modal(0);
        }),
    );

    let _ = dlg.show_modal();
    let updated = result.borrow().clone();
    updated
}

// ─── Main ──────────────────────────────────────────────────────────────────
fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("ru_wx Cloud Demo")
        .with_size(720, 480)
        .build();

    let hwnd = frame.hwnd();

    let cfg = Rc::new(RefCell::new(load_config()));
    let initial_provider = provider_from_str(&cfg.borrow().provider);
    let state = Rc::new(RefCell::new(UiState {
        provider: initial_provider,
        current_path: String::new(),
    }));

    // Cache degli elementi correnti (indice = riga della lista).
    let items_cache: Rc<RefCell<Vec<Item>>> = Rc::new(RefCell::new(Vec::new()));

    // Header che mostra provider + percorso corrente.
    let header = StaticText::new(&frame, "Caricamento…");

    // Lista report: Name | Type | Size.
    let list = ListCtrl::new(&frame, ListCtrlStyle::Report);
    list.insert_column(0, "Nome", 320);
    list.insert_column(1, "Tipo", 100);
    list.insert_column(2, "Dimensione", 120);

    // Riga di pulsanti.
    let mut row = BoxSizer::horizontal();

    let cfg_for_refresh = Rc::clone(&cfg);
    let state_for_refresh = Rc::clone(&state);
    let btn_refresh = Button::new(&frame, "Aggiorna");
    btn_refresh.on_click(&frame, move || {
        do_refresh(&cfg_for_refresh, &state_for_refresh, hwnd)
    });
    row.add(btn_refresh.as_widget_ref());

    let btn_up = Button::new(&frame, "Su");
    let cfg_for_up = Rc::clone(&cfg);
    let state_for_up = Rc::clone(&state);
    btn_up.on_click(&frame, move || {
        let (cfg_snap, provider, parent) = {
            let c = cfg_for_up.borrow();
            let s = state_for_up.borrow();
            let parent = parent_path(s.provider, &s.current_path);
            (c.clone(), s.provider, parent)
        };
        if let Some(parent) = parent {
            state_for_up.borrow_mut().current_path = parent.clone();
            spawn_list(hwnd, cfg_snap, provider, parent);
        }
    });
    row.add(btn_up.as_widget_ref());

    let list_for_open = list.clone();
    let cache_for_open = Rc::clone(&items_cache);
    let cfg_for_open = Rc::clone(&cfg);
    let state_for_open = Rc::clone(&state);
    let btn_open = Button::new(&frame, "Apri");
    btn_open.on_click(&frame, move || {
        let Some(item) = selected_item(&list_for_open, &cache_for_open) else {
            return;
        };
        if !item.is_dir {
            return;
        }
        let cfg_snap = cfg_for_open.borrow().clone();
        let provider = state_for_open.borrow().provider;
        state_for_open.borrow_mut().current_path = item.path.clone();
        spawn_list(hwnd, cfg_snap, provider, item.path);
    });
    row.add(btn_open.as_widget_ref());

    let cfg_for_mkdir = Rc::clone(&cfg);
    let state_for_mkdir = Rc::clone(&state);
    let frame_for_mkdir = frame.clone();
    let btn_mkdir = Button::new(&frame, "Nuova cartella");
    btn_mkdir.on_click(&frame, move || {
        let Some(name) = TextEntryDialog::new(
            &frame_for_mkdir,
            "Nome della nuova cartella:",
            "Nuova cartella",
            "",
        )
        .show_modal() else {
            return;
        };
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let (cfg_snap, provider, folder_path) = {
            let c = cfg_for_mkdir.borrow();
            let s = state_for_mkdir.borrow();
            let folder_path = join_path(s.provider, &s.current_path, &name);
            (c.clone(), s.provider, folder_path)
        };
        spawn_op_then_list(
            hwnd,
            cfg_snap,
            provider,
            folder_path.clone(),
            "Nuova cartella".into(),
            move |c| create_folder_remote(c, provider, &folder_path),
        );
    });
    row.add(btn_mkdir.as_widget_ref());

    let cfg_for_upload = Rc::clone(&cfg);
    let state_for_upload = Rc::clone(&state);
    let frame_for_upload = frame.clone();
    let btn_upload = Button::new(&frame, "Carica file");
    btn_upload.on_click(&frame, move || {
        let mut dlg = FileDialog::new(&frame_for_upload, FileDialogStyle::Open);
        let Some(local) = dlg.show_modal() else {
            return;
        };
        let local = PathBuf::from(local);
        let Some(name) = local.file_name().map(|n| n.to_string_lossy().to_string()) else {
            return;
        };
        let (cfg_snap, provider, dest) = {
            let c = cfg_for_upload.borrow();
            let s = state_for_upload.borrow();
            let dest = join_path(s.provider, &s.current_path, &name);
            (c.clone(), s.provider, dest)
        };
        spawn_op_then_list(
            hwnd,
            cfg_snap,
            provider,
            dest.clone(),
            "Carica file".into(),
            move |c| upload_remote(c, provider, &local, &dest),
        );
    });
    row.add(btn_upload.as_widget_ref());

    let mut root_sizer = BoxSizer::vertical();
    root_sizer.add(header.as_widget_ref());
    root_sizer.add_with_proportion(list.as_widget_ref(), 1);
    root_sizer.add_sizer(row);
    frame.set_sizer(root_sizer);

    // ─── Menu ──────────────────────────────────────────────────────────────
    let mut cloud_menu = Menu::new("&Cloud");

    let cfg_for_settings = Rc::clone(&cfg);
    let state_for_settings = Rc::clone(&state);
    let frame_for_settings = frame.clone();
    cloud_menu.append("&Impostazioni…", &frame, move || {
        let current = cfg_for_settings.borrow().clone();
        let updated = show_settings(&frame_for_settings, &current);
        let provider = provider_from_str(&updated.provider);
        *cfg_for_settings.borrow_mut() = updated;
        state_for_settings.borrow_mut().provider = provider;
        do_refresh(&cfg_for_settings, &state_for_settings, hwnd);
    });

    cloud_menu.append_separator();

    let cfg_for_dbx = Rc::clone(&cfg);
    let state_for_dbx = Rc::clone(&state);
    cloud_menu.append("Usa &Dropbox", &frame, move || {
        cfg_for_dbx.borrow_mut().provider = "dropbox".to_string();
        save_config(&cfg_for_dbx.borrow());
        state_for_dbx.borrow_mut().provider = Provider::Dropbox;
        state_for_dbx.borrow_mut().current_path = String::new();
        do_refresh(&cfg_for_dbx, &state_for_dbx, hwnd);
    });

    let cfg_for_dav = Rc::clone(&cfg);
    let state_for_dav = Rc::clone(&state);
    cloud_menu.append("Usa &WebDAV", &frame, move || {
        cfg_for_dav.borrow_mut().provider = "webdav".to_string();
        save_config(&cfg_for_dav.borrow());
        state_for_dav.borrow_mut().provider = Provider::Webdav;
        state_for_dav.borrow_mut().current_path = String::new();
        do_refresh(&cfg_for_dav, &state_for_dav, hwnd);
    });

    let mut actions_menu = Menu::new("&Azioni");

    let cfg_for_dl = Rc::clone(&cfg);
    let state_for_dl = Rc::clone(&state);
    let list_for_dl = list.clone();
    let cache_for_dl = Rc::clone(&items_cache);
    let frame_for_dl = frame.clone();
    actions_menu.append("&Scarica", &frame, move || {
        let Some(item) = selected_item(&list_for_dl, &cache_for_dl) else {
            return;
        };
        if item.is_dir {
            return;
        }
        let mut dlg = FileDialog::new(&frame_for_dl, FileDialogStyle::Save);
        if let Some(local) = dlg.show_modal() {
            let (cfg_snap, provider) = {
                let c = cfg_for_dl.borrow();
                (c.clone(), state_for_dl.borrow().provider)
            };
            let path = item.path.clone();
            let local = PathBuf::from(local);
            std::thread::spawn(move || {
                let result = download_remote(&cfg_snap, provider, &path)
                    .and_then(|bytes| std::fs::write(&local, bytes).map_err(|e| e.to_string()));
                let payload = Box::new(OpPayload {
                    title: "Scarica".into(),
                    message: match result {
                        Ok(()) => format!("Salvato in {}", local.display()),
                        Err(e) => format!("Download fallito:\n- {e}"),
                    },
                });
                Frame::post_raw_message_ptr(
                    hwnd,
                    MSG_OP,
                    Box::into_raw(payload) as *mut std::ffi::c_void,
                );
            });
        }
    });

    let cfg_for_rename = Rc::clone(&cfg);
    let state_for_rename = Rc::clone(&state);
    let list_for_rename = list.clone();
    let cache_for_rename = Rc::clone(&items_cache);
    let frame_for_rename = frame.clone();
    actions_menu.append("&Rinomina…", &frame, move || {
        let Some(item) = selected_item(&list_for_rename, &cache_for_rename) else {
            return;
        };
        let Some(new_name) =
            TextEntryDialog::new(&frame_for_rename, "Nuovo nome:", "Rinomina", &item.name)
                .show_modal()
        else {
            return;
        };
        let new_name = new_name.trim().to_string();
        if new_name.is_empty() || new_name == item.name {
            return;
        }
        let (cfg_snap, provider, current, from, to) = {
            let c = cfg_for_rename.borrow();
            let s = state_for_rename.borrow();
            let to = join_path(s.provider, &s.current_path, &new_name);
            (
                c.clone(),
                s.provider,
                s.current_path.clone(),
                item.path.clone(),
                to,
            )
        };
        spawn_op_then_list(
            hwnd,
            cfg_snap,
            provider,
            current,
            "Rinomina".into(),
            move |c| rename_remote(c, provider, &from, &to),
        );
    });

    let cfg_for_delete = Rc::clone(&cfg);
    let state_for_delete = Rc::clone(&state);
    let list_for_delete = list.clone();
    let cache_for_delete = Rc::clone(&items_cache);
    let frame_for_delete = frame.clone();
    actions_menu.append("&Elimina", &frame, move || {
        let Some(item) = selected_item(&list_for_delete, &cache_for_delete) else {
            return;
        };
        if message_box(
            &frame_for_delete,
            &format!("Eliminare \"{}\"?", item.name),
            "Elimina",
            MessageBoxStyle::YesNo,
            MessageBoxIcon::Warning,
        ) != ru_wx::MessageBoxResult::Yes
        {
            return;
        }
        let (cfg_snap, provider, current, path) = {
            let c = cfg_for_delete.borrow();
            let s = state_for_delete.borrow();
            (
                c.clone(),
                s.provider,
                s.current_path.clone(),
                item.path.clone(),
            )
        };
        spawn_op_then_list(
            hwnd,
            cfg_snap,
            provider,
            current,
            "Elimina".into(),
            move |c| delete_remote(c, provider, &path),
        );
    });

    let mut menubar = MenuBar::new();
    menubar.append(cloud_menu);
    menubar.append(actions_menu);
    frame.set_menu_bar(menubar);

    // ─── Handler risultati (worker → UI) ───────────────────────────────────
    let list_for_msg = list.clone();
    let header_for_msg = header.clone();
    let cache_for_msg = Rc::clone(&items_cache);
    let state_for_msg = Rc::clone(&state);
    let frame_for_msg = frame.clone();
    frame.register_raw_message_handler(MSG_LIST, move |w, l| {
        let lo = (w as u64) & 0xFFFF_FFFF;
        let hi = (l as u64) & 0xFFFF_FFFF;
        let ptr = ((hi << 32) | lo) as *mut ListPayload;
        if ptr.is_null() {
            return;
        }
        let payload = unsafe { Box::from_raw(ptr) };

        state_for_msg.borrow_mut().provider = payload.provider;
        state_for_msg.borrow_mut().current_path = payload.path.clone();

        let label = format!(
            "{} — {}",
            match payload.provider {
                Provider::Dropbox => "Dropbox",
                Provider::Webdav => "WebDAV",
            },
            if payload.path.is_empty() {
                "/"
            } else {
                &payload.path
            }
        );
        header_for_msg.set_label(&label);
        frame_for_msg.set_title(&format!("ru_wx Cloud Demo — {label}"));

        if let Some(err) = &payload.error {
            *cache_for_msg.borrow_mut() = Vec::new();
            populate_list(&list_for_msg, &[]);
            let _ = message_box(
                &frame_for_msg,
                &format!("Impossibile leggere la cartella:\n- {err}"),
                "Cloud",
                MessageBoxStyle::Ok,
                MessageBoxIcon::Error,
            );
            return;
        }

        *cache_for_msg.borrow_mut() = payload.items.clone();
        populate_list(&list_for_msg, &payload.items);
    });

    let frame_for_op = frame.clone();
    frame.register_raw_message_handler(MSG_OP, move |w, l| {
        let lo = (w as u64) & 0xFFFF_FFFF;
        let hi = (l as u64) & 0xFFFF_FFFF;
        let ptr = ((hi << 32) | lo) as *mut OpPayload;
        if ptr.is_null() {
            return;
        }
        let payload = unsafe { Box::from_raw(ptr) };
        let _ = message_box(
            &frame_for_op,
            &payload.message,
            &payload.title,
            MessageBoxStyle::Ok,
            MessageBoxIcon::Information,
        );
    });

    // ─── Avvio ─────────────────────────────────────────────────────────────
    do_refresh(&cfg, &state, hwnd);
    app.run(frame);
}
