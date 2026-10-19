//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `Widget::bounds_screen()` (Livello 2) — rect in coordinate
//! schermo della frame e di un figlio, misurati dopo la mappatura.
//!
//! Demonstrates:
//! - `widget_screen_rect(frame.hwnd())` con origine schermo (X11) + `compute_bounds`
//! - `Button::bounds_screen()` dentro il rettangolo della frame
//!
//! Run with:
//! ```bash
//! cargo run --example mt_bounds_screen
//! ```

#![windows_subsystem = "windows"]

use std::cell::Cell;
use std::rc::Rc;

use ru_wx::platform::widget_screen_rect;
use ru_wx::{App, BoxSizer, Button, Frame, StaticText};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — bounds_screen")
        .with_size(480, 320)
        .build();

    let hint = StaticText::new(&frame, "Verifica rect in coordinate schermo.");
    let button = Button::new(&frame, "Premi");

    let mut sizer = BoxSizer::vertical();
    sizer.add(hint.as_widget_ref());
    sizer.add(button.as_widget_ref());
    frame.set_sizer(sizer);

    // Le verifiche girano in `on_idle`: su Linux `Frame::show()` mappa la
    // finestra (`present_frame` + `pump`) prima del loop idle, quindi a
    // quel punto `compute_bounds` e la posizione X11 sono valide.
    let checked = Rc::new(Cell::new(false));
    let checked_for_idle = Rc::clone(&checked);
    let frame_for_idle = frame.clone();
    frame.on_idle(move |_| {
        if checked_for_idle.get() {
            return;
        }
        checked_for_idle.set(true);

        let frame_rect = widget_screen_rect(frame_for_idle.hwnd());
        println!(
            "BOUNDS frame=({}, {} {}x{})",
            frame_rect.x, frame_rect.y, frame_rect.width, frame_rect.height
        );
        assert!(frame_rect.width > 0, "frame width > 0");
        assert!(frame_rect.height > 0, "frame height > 0");

        let btn_widget = button.as_widget_ref();
        let btn_rect = btn_widget.borrow().bounds_screen();
        println!(
            "BUTTON button=({}, {} {}x{})",
            btn_rect.x, btn_rect.y, btn_rect.width, btn_rect.height
        );
        assert!(btn_rect.width > 0, "button width > 0");
        assert!(btn_rect.height > 0, "button height > 0");

        // Il figlio deve cadere dentro la frame (coord. schermo).
        assert!(
            btn_rect.x >= frame_rect.x && btn_rect.y >= frame_rect.y,
            "button origin inside frame"
        );
        assert!(
            btn_rect.x as i64 + i64::from(btn_rect.width)
                <= frame_rect.x as i64 + i64::from(frame_rect.width),
            "button right edge inside frame"
        );
        assert!(
            btn_rect.y as i64 + i64::from(btn_rect.height)
                <= frame_rect.y as i64 + i64::from(frame_rect.height),
            "button bottom edge inside frame"
        );

        println!("BOUNDS_MINITEST_OK");
    });

    app.run(frame);
}
