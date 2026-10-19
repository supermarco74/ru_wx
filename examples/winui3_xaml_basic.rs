//! WinUI3_Xaml Basic Example
//!
//! Demonstrates the winui3_xaml backend which uses WinAppSDK bootstrap
//! with real WinRT/XAML controls via windows-reactor.
//!
//! Key differences from other backends:
//! - win32: Standard Win32 API (CreateWindowExW)
//! - winui3: Win32 modern (builder pattern, more controls)
//! - winui3_xaml: Real WinUI3/XAML via windows-reactor (WinRT bindings)
//!
//! Run with: cargo run --example winui3_xaml_basic --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame};

fn main() {
    println!("Starting WinUI3_Xaml Basic Example...");
    println!("This uses WinAppSDK bootstrap + real WinRT/XAML via reactor");

    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("WinUI3_Xaml Backend")
        .with_size(600, 400)
        .build();

    println!("Window created with XAML backend");
    println!("The X button CLOSES the window");

    match app.run(frame) {
        Ok(_) => println!("Application closed normally"),
        Err(e) => eprintln!("Error: {:?}", e),
    }
}
