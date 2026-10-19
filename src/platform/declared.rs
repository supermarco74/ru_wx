//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Registro dei widget "puro Rust" dichiarati per l'inspector.
//!
//! I container senza controllo nativo proprio (es. [`crate::containers::simplebook::Simplebook`])
//! non hanno una classe Win32/NSView da cui l'inspector possa dedurre il
//! ruolo: qui registrano **ruolo**, **selezione corrente** e un eventuale
//! selettore per `inject_event` (cfr. §6 di `PIANO_PORTABILITA_COMPONENTI.md`).
//!
//! Le chiavi sono gli handle nativi (`NativeHandle`), gli stessi usati come
//! `id` dei nodi dell'albero. I job dell'inspector girano sul thread UI,
//! quindi il `thread_local` è sempre coerente con lo stato dei widget.
//! Il modulo è compilato sempre (nessuna cfg), così il codice dei
//! container non ha bisogno di ramo no-op per la release.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::platform::NativeHandle;

/// Widget dichiarato: ruolo logico + stato esposto in `read_state`.
struct DeclaredWidget {
    /// Ruolo logico condiviso (§6 del piano), es. `"tabs"`.
    role: &'static str,
    /// Selezione corrente (campo `selection` di `read_state`).
    selection: usize,
    /// Selettore invocato da `inject_event` con azione `select`.
    selector: Option<Rc<dyn Fn(usize)>>,
}

thread_local! {
    static DECLARED: RefCell<HashMap<NativeHandle, DeclaredWidget>> =
        RefCell::new(HashMap::new());
}

/// Registra il ruolo dichiarato di un widget puro Rust.
///
/// Il widget deve chiamarla alla creazione: è l'unico modo per far
/// rispondere `/find?role=…` su un contenitore senza classe nativa.
pub(crate) fn mark(handle: NativeHandle, role: &'static str) {
    DECLARED.with(|map| {
        let mut map = map.borrow_mut();
        match map.get_mut(&handle) {
            Some(widget) => widget.role = role,
            None => {
                map.insert(
                    handle,
                    DeclaredWidget {
                        role,
                        selection: 0,
                        selector: None,
                    },
                );
            }
        }
    });
}

/// Registra la selezione corrente del widget (da richiamare a ogni
/// cambio di pagina, perché è `read_state` a leggerla).
pub(crate) fn set_selection(handle: NativeHandle, selection: usize) {
    DECLARED.with(|map| {
        if let Some(widget) = map.borrow_mut().get_mut(&handle) {
            widget.selection = selection;
        }
    });
}

/// Registra il selettore usato da `inject_event` (`select`).
pub(crate) fn set_selector(handle: NativeHandle, selector: Rc<dyn Fn(usize)>) {
    DECLARED.with(|map| {
        if let Some(widget) = map.borrow_mut().get_mut(&handle) {
            widget.selector = Some(selector);
        }
    });
}

/// Ruolo dichiarato del widget, se registrato.
pub(crate) fn role(handle: NativeHandle) -> Option<&'static str> {
    DECLARED.with(|map| map.borrow().get(&handle).map(|w| w.role))
}

/// Selezione dichiarata del widget, se registrato.
pub(crate) fn selection(handle: NativeHandle) -> Option<usize> {
    DECLARED.with(|map| map.borrow().get(&handle).map(|w| w.selection))
}

/// Applica la selezione tramite il selettore del widget.
///
/// Ritorna `false` se il widget non ne espone uno: `inject_event` in
/// quel caso segnala l'errore invece di fingere il successo. Il
/// selettore è clonato *prima* di invocarlo, perché aggiornando a sua
/// volta la selezione entrerebbe nel `RefCell` del registro.
pub(crate) fn select(handle: NativeHandle, index: usize) -> bool {
    let selector = DECLARED.with(|map| map.borrow().get(&handle).and_then(|w| w.selector.clone()));
    match selector {
        Some(selector) => {
            selector(index);
            true
        }
        None => false,
    }
}
