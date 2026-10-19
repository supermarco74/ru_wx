//! Demo stabile per il Debug Inspector (cfr. `PIANO_DEBUG_INSPECTOR.md` §16).
//!
//! È il "terreno di prova" end-to-end: contiene un **form** (label, campi
//! di testo, password, checkbox, choice) e una **tabella** (`ListCtrl`
//! report). Serve a verificare le tre capacità dell'inspector:
//!
//! - **Stato** — `GET /tree` / `/find` (o i tool MCP `get_ui_tree`,
//!   `find_element`): tipo, testo, bounds reali, colori, stato.
//! - **Azione** — `POST /event` / `interact_with_element`: click sul
//!   pulsante *Login*, scrittura nei campi, selezione nella tabella.
//! - **Visione** — `GET /screenshot` / `take_screenshot`: PNG on-demand.
//!
//! Run with:
//! ```bash
//! cargo run --example inspector_demo
//! # poi, da un altro terminale:
//! curl -s localhost:7373/health
//! curl -s localhost:7373/tree | jq .
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{
    App, BoxSizer, Button, CheckBox, Choice, Frame, ListCtrl, ListCtrlStyle, StaticText, TextCtrl,
};

const ROLES: [&str; 3] = ["Admin", "Editor", "Viewer"];

/// (nome, ruolo, stato) — contenuto iniziale della tabella.
const USERS: &[(&str, &str, &str)] = &[
    ("alice", "Admin", "active"),
    ("bob", "Editor", "active"),
    ("carol", "Viewer", "suspended"),
    ("dave", "Viewer", "active"),
];

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("ru_wx Inspector — Demo")
        .with_size(640, 560)
        .build();

    // ── Form di login ───────────────────────────────────────────────
    let heading = StaticText::new(&frame, "Sign in");

    let lbl_user = StaticText::new(&frame, "Username:");
    let user = TextCtrl::new(&frame, "alice");

    let lbl_pass = StaticText::new(&frame, "Password:");
    let pass = TextCtrl::password(&frame, "");

    let remember = CheckBox::new(&frame, "Remember me");
    remember.set_checked(true);

    let lbl_role = StaticText::new(&frame, "Role:");
    let role = Choice::new(&frame);
    for r in ROLES {
        role.append(r);
    }
    role.set_selection(0);

    let login = Button::new(&frame, "Login");
    let result = StaticText::new(&frame, "Not signed in.");

    // Al click: aggiorna la label con i valori *reali* letti dai widget.
    {
        let user = user.clone();
        let pass = pass.clone();
        let role = role.clone();
        let remember = remember.clone();
        let result = result.clone();
        login.on_click(&frame, move || {
            let who = user.get_value();
            let who = if who.trim().is_empty() {
                "(empty)".to_string()
            } else {
                who
            };
            let role = role
                .get_selection()
                .and_then(|i| role.get_string(i))
                .unwrap_or_else(|| "?".to_string());
            result.set_label(&format!(
                "Signed in as {who} [{role}] — password {} chars, remember={}",
                pass.get_value().chars().count(),
                remember.is_checked()
            ));
        });
    }

    // ── Tabella utenti (ListCtrl report) ────────────────────────────
    let lbl_table = StaticText::new(&frame, "Users:");
    let table = ListCtrl::new(&frame, ListCtrlStyle::Report);
    table.insert_column(0, "Name", 200);
    table.insert_column(1, "Role", 140);
    table.insert_column(2, "Status", 140);
    for (i, (name, user_role, state)) in USERS.iter().enumerate() {
        table.insert_item(i, name);
        table.set_item_text(i, 1, user_role);
        table.set_item_text(i, 2, state);
    }

    let add_row = Button::new(&frame, "Add row");
    {
        let table = table.clone();
        add_row.on_click(&frame, move || {
            let n = table.get_item_count();
            let name = format!("user-{:02}", n + 1);
            table.insert_item(n, &name);
            table.set_item_text(n, 1, "Viewer");
            table.set_item_text(n, 2, "pending");
        });
    }

    // ── Layout ──────────────────────────────────────────────────────
    let mut form = BoxSizer::vertical();
    form.add(heading.as_widget_ref());
    form.add(lbl_user.as_widget_ref());
    form.add(user.as_widget_ref());
    form.add(lbl_pass.as_widget_ref());
    form.add(pass.as_widget_ref());
    form.add(remember.as_widget_ref());
    form.add(lbl_role.as_widget_ref());
    form.add(role.as_widget_ref());
    form.add(login.as_widget_ref());
    form.add(result.as_widget_ref());
    form.add(lbl_table.as_widget_ref());
    form.add_with_proportion(table.as_widget_ref(), 1);
    form.add(add_row.as_widget_ref());

    frame.set_sizer(form);
    app.run(frame);
}
