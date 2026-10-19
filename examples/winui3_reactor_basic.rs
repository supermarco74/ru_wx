//! WinUI3 Reactor Example - Native Fluent UI
//!
//! Uses windows-reactor from windows-rs for true WinUI3 with:
//! - Fluent Design (Mica backdrop)
//! - Reactive/React-like component model
//! - Native WinRT widget controls
//!
//! This is the REAL WinUI3 implementation, not Win32 wrapped in WinAppSDK.
//!
//! Run with: cargo run --example winui3_reactor_basic --features winui3_reactor
//! Requires: WinAppSDK installed on the system

#![cfg(target_os = "windows")]

use windows_reactor::*;

fn app(cx: &mut RenderCx) -> Element {
    // State hooks like React
    let (count, set_count) = cx.use_state(0_i32);
    let (is_bold, set_bold) = cx.use_state(false);

    // Build UI with reactive state
    vstack((
        // Title
        text_block("WinUI3 Reactor Demo")
            .font_size(28.0)
            .bold()
            .horizontal_alignment(HorizontalAlignment::Center),
        // Counter
        text_block(format!("Count: {count}"))
            .font_size(18.0)
            .horizontal_alignment(HorizontalAlignment::Center),
        // Buttons row - use .setter() to avoid clone issues
        hstack((
            button("Increment").on_click(set_count.setter(count + 1)),
            button("Decrement").on_click(set_count.setter(count - 1)),
            button("Reset").on_click(set_count.setter(0)),
        ))
        .spacing(8.0),
        // Toggle for styling - use .setter() helper
        toggle_button("Bold Text", is_bold).on_checked(set_bold),
        // Styled text - use conditional rendering
        if is_bold {
            text_block("Style: Bold").font_size(14.0).bold()
        } else {
            text_block("Style: Normal").font_size(14.0)
        },
        // Info
        text_block("Built with windows-reactor + WinRT")
            .font_size(12.0)
            .foreground(Color::rgb(128, 128, 128)),
    ))
    .spacing(16.0)
    .padding(24.0)
    .vertical_alignment(VerticalAlignment::Center)
    .horizontal_alignment(HorizontalAlignment::Center)
    .into()
}

fn main() -> Result<()> {
    println!("Starting WinUI3 Reactor Demo...");
    println!("This uses TRUE WinUI3 with Fluent Design!");

    // Bootstrap WinAppSDK
    bootstrap()?;

    // Run the reactive app
    App::new()
        .title("WinUI3 Reactor Demo")
        .inner_size(500.0, 450.0)
        .backdrop(Backdrop::Mica)
        .render(app)
}
