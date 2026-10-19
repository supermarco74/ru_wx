//! Esempio di utilizzo del Modern Window API
//!
//! Questo esempio dimostra l'uso del nuovo modulo `window.rs`
//! che implementa i pattern moderni da windows-rs:
//! - WindowBuilder con chaining dei builder method
//! - run(), run_with(), pump() per il message loop
//! - MessageHandler per intercettare messaggi raw
//! - Result<T> per error handling

use ru_wx::platform::winui3::app::{Button, CheckBox, Panel, StaticText, TextCtrl};
use ru_wx::platform::winui3::common::WidgetCommon;
use ru_wx::platform::winui3::window::{run, wm, ws, Window, WindowBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creazione finestra con Modern Window API...");

    // Creazione della finestra con builder pattern
    let window = WindowBuilder::new("Modern Window API Demo")
        .size(800, 600)
        .position(100, 100)
        .style(ws::OVERLAPPEDWINDOW | ws::VISIBLE)
        .on_message(move |hwnd, msg, _wparam, _lparam| {
            match msg {
                // Gestiamo WM_CLOSE per chiudere la finestra
                wm::CLOSE => {
                    println!("WM_CLOSE ricevuto - chiudo la finestra");
                    // Usa PostQuitMessage per uscire dal message loop
                    unsafe {
                        use windows_sys::Win32::UI::WindowsAndMessaging::{
                            DestroyWindow, PostQuitMessage,
                        };
                        DestroyWindow(hwnd);
                        PostQuitMessage(0);
                    }
                    Some(0)
                }
                // Gestiamo WM_DESTROY
                wm::DESTROY => {
                    println!("WM_DESTROY ricevuto");
                    None
                }
                _ => None,
            }
        })
        .on_resize(|width, height| {
            println!("Resize: {}x{}", width, height);
        })
        .create()?;

    println!("Finestra creata con successo!");
    println!("HWND: {:?}", window.hwnd());
    println!("Client size: {:?}", window.client_size());

    // Aggiungiamo controlli alla finestra usando il modulo app
    add_controls(&window);

    // Centro la finestra
    window.center();

    // Messaggio loop blocking
    println!("Entrando nel message loop...");
    run();

    println!("Message loop terminato.");
    Ok(())
}

/// Aggiunge controlli alla finestra usando i widget del modulo app
fn add_controls(window: &Window) {
    // Creiamo un pannello principale
    let panel = Panel::new(window);
    panel.set_rect(10, 10, 760, 540);

    // Aggiungiamo una label
    let label = StaticText::new(&panel, "Benvenuto nel Modern Window API!");
    label.set_position(20, 20);
    label.set_size(300, 25);

    // Aggiungiamo un pulsante
    let button = Button::new(&panel, "Cliccami!");
    button.set_position(20, 60);
    button.set_size(120, 35);

    // Aggiungiamo un TextCtrl
    let text = TextCtrl::new(&panel, "Inserisci del testo qui...");
    text.set_position(20, 110);
    text.set_size(300, 25);

    // Aggiungiamo una checkbox
    let checkbox = CheckBox::new(&panel, "Accetta i termini");
    checkbox.set_position(20, 150);

    println!("Controlli aggiunti alla finestra");
}
