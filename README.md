gui library in rust porting  and implementation from  wxwingets
# ru_wx
more detailed on
https://ru_wx.easytaskflow.app/

<img width="780" height="580" alt="minieditor_esempio" src="https://github.com/user-attachments/assets/36757fb4-4139-4adf-aa36-9ad0594992a8" />


**ru_wx** is a cross-platform GUI library written in **pure Rust**, with a [wxWidgets](https://www.wxwidgets.org/)-inspired API and native platform controls.

Unlike Rust bindings to wxWidgets, **ru_wx has no C++ dependency**: it talks directly to the OS APIs (Win32 on Windows) while keeping a familiar API for wxWidgets users.

## Features

- wxWidgets-like API (`Frame`, `Button`, `Grid`, `TreeCtrl`, `AuiToolBar`, …)
- Native Win32 widgets (HWND-based) on Windows
- No C++ or wxWidgets bindings
- Modular layout: controls, windows, containers, drawing, dialogs, drag-and-drop
- 40+ examples and focused minitests per migrated component
- CI on Windows, macOS, and Linux
- ~90% coverage of the common wxWidgets surface on Win32

## Platform support

| Platform | Backend   | Status   |
|----------|-----------|----------|
| Windows  | Win32 API | Active   |
| macOS    | AppKit    | Planned  |
| Linux    | GTK       | Planned  |

## Requirements

- [Rust](https://rustup.rs/) 1.70+ (edition 2021)
- Windows: MSVC toolchain

## Quick start

Add to your `Cargo.toml`:

```toml
[dependencies]
ru_wx = "0.6.4"



_______________________________________________________________
-------------------ITALIANO
______________________________________________________________
**ru_wx** è una libreria GUI cross-platform scritta in **Rust puro**, con un'API ispirata a [wxWidgets](https://www.wxwidgets.org/) e controlli nativi su ogni piattaforma.

A differenza dei binding Rust verso wxWidgets, **ru_wx non usa C++**: implementa direttamente le API di sistema (Win32 su Windows) mantenendo un'interfaccia familiare per chi conosce wxWidgets.

## Caratteristiche

- API simile a wxWidgets (`Frame`, `Button`, `Grid`, `TreeCtrl`, `AuiToolBar`, …)
- Widget nativi Win32 (HWND) su Windows
- Nessuna dipendenza da C++ o da wxWidgets
- Moduli organizzati per dominio: controlli, finestre, layout, drawing, dialoghi, DnD
- Oltre 40 esempi e minitest per ogni componente migrato
- CI su Windows, macOS e Linux
- Copertura stimata ~90% della superficie wxWidgets comune su Win32

## Piattaforme

| Piattaforma | Backend        | Stato        |
|-------------|----------------|--------------|
| Windows     | Win32 API      | Attivo       |
| macOS       | AppKit         | Pianificato  |
| Linux       | GTK            | Pianificato  |

## Requisiti

- [Rust](https://rustup.rs/) 1.70+ (edition 2021)
- Windows: MSVC toolchain

## Quick start

Aggiungi al tuo `Cargo.toml`:

```toml
[dependencies]
ru_wx = "0.6.4"
