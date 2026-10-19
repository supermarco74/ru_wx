//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `BreadcrumbBar` — barra di navigazione a briciole di pane.
//!
//! La classe nativa `msctls_breadcrumb32` (e le API `BREADCRUMBITEM`,
//! `BCM_SETITEMS`, `BCN_GETITEMS`) non è esposta da windows-sys 0.59 e la
//! variante nativa richiede item PIDL: il controllo viene quindi creato
//! comunque con la **fallback a item testuali** documentata in
//! `ru_wx::controls::breadcrumb_bar` (contenitore `STATIC` + un pulsante per
//! segmento, routing `WM_COMMAND` via sub-classing).
//!
//! Dimostra:
//! - creazione della barra e impostazione del percorso iniziale
//!   `C:\Program Files\ru_wx\docs`;
//! - click su un segmento → aggiornamento di un `StaticText`;
//! - lettura di `current_path()`.
//!
//! Run con:
//! ```bash
//! cargo run --example mt_breadcrumb
//! ```

#![windows_subsystem = "windows"]

use ru_wx::controls::breadcrumb_bar::BreadcrumbBar;
use ru_wx::{App, BoxSizer, Frame, StaticText};

const INITIAL_PATH: &str = r"C:\Program Files\ru_wx\docs";

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — BreadcrumbBar")
        .with_size(700, 320)
        .build();

    let hint = StaticText::new(&frame, "Percorso iniziale: C:\\Program Files\\ru_wx\\docs");
    let bar = BreadcrumbBar::new(&frame);
    bar.set_path(INITIAL_PATH);

    let esito = StaticText::new(&frame, "Clicca un segmento della breadcrumb…");
    let corrente = StaticText::new(&frame, &format!("current_path(): {}", bar.current_path()));

    // Click su un segmento → indice + radice del percorso fino a lì.
    {
        let esito = esito.clone();
        let corrente = corrente.clone();
        let bar_c = bar.clone();
        bar.on_navigate(&frame, move |idx, path| {
            esito.set_label(&format!("Navigazione al segmento {idx}: {path}"));
            corrente.set_label(&format!("current_path(): {}", bar_c.current_path()));
        });
    }

    let mut sizer = BoxSizer::vertical();
    sizer.add(hint.as_widget_ref());
    sizer.add(bar.as_widget_ref());
    sizer.add(esito.as_widget_ref());
    sizer.add(corrente.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
