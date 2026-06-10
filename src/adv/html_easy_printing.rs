//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Print HTML content (`wxHtmlEasyPrinting`).

use crate::adv::html_window::HtmlWindow;
use crate::printing::{Printout, Printer};
use crate::window::frame::Frame;

/// Simple HTML-to-printer helper (`wxHtmlEasyPrinting`).
pub struct HtmlEasyPrinting {
    name: String,
}

impl HtmlEasyPrinting {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn print_html(&self, frame: &Frame, html: &str) -> bool {
        let mut html_win = HtmlWindow::new(frame);
        html_win.set_page(html);
        let _ = html_win;
        let mut printer = Printer::new();
        printer.print(&Printout::new(&self.name, 1))
    }

    pub fn preview_html(&self, html: &str) -> String {
        let plain = html.replace("<br>", "\n").replace("<p>", "").replace("</p>", "\n");
        format!("Preview:\n{plain}")
    }
}
