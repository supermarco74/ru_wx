//! Sistema di eventi di basso livello cross-platform.
//!
//! Questo modulo fornisce un'API unificata per gestire:
//! - Eventi hover (mouse enter/exit/move)
//! - Eventi di temporizzazione (hover timer per tooltip)
//! - Click avanzati (double-click, right-click, middle-click)
//! - Drag & drop
//! - Rotella mouse
//! - Touch events
//! - Gesture events
//!
//! ## Architettura
//!
//! Su **macOS** usa `NSTrackingArea` per eventi nativi.
//! Su **Windows** usa `TrackMouseEvent` per eventi nativi.
//!
//! ## Esempio d'uso
//!
//! ```ignore
//! use crate::core::system_events::{SystemEvent, HoverHandler};
//!
//! struct MyHandler;
//! impl HoverHandler for MyHandler {
//!     fn on_hover_enter(&mut self, widget: u64, x: i32, y: i32) {
//!         println!("Hover enter at ({}, {})", x, y);
//!     }
//!     fn on_hover_exit(&mut self, widget: u64) {
//!         println!("Hover exit");
//!     }
//! }
//! ```

/// Identificativo del widget (handle interno).
pub type WidgetId = u64;

/// Identificativo per un touch point.
#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    /// ID univoco del touch (finger ID su iOS/macOS).
    pub id: u64,
    /// Posizione X.
    pub x: f64,
    /// Posizione Y.
    pub y: f64,
    /// Pressione normalizzata (0.0 - 1.0).
    pub pressure: f64,
}

impl TouchPoint {
    /// Crea un nuovo touch point.
    pub fn new(id: u64, x: f64, y: f64) -> Self {
        Self {
            id,
            x,
            y,
            pressure: 1.0,
        }
    }

    /// Crea un touch point con pressione.
    pub fn with_pressure(id: u64, x: f64, y: f64, pressure: f64) -> Self {
        Self { id, x, y, pressure }
    }
}

/// Tipo di gesture riconosciuta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GestureType {
    /// Pinch gesture (zoom in/out).
    Pinch,
    /// Rotazione con due dita.
    Rotate,
    /// Pan/swipe gesture.
    Pan,
    /// Tap gesture.
    Tap,
    /// Long press.
    LongPress,
}

/// Informazioni su un gesture.
#[derive(Debug, Clone)]
pub struct GestureInfo {
    /// Tipo di gesture.
    pub gesture_type: GestureType,
    /// Scala rispetto all'inizio del gesture (per Pinch).
    pub scale: f64,
    /// Rotazione in radianti (per Rotate).
    pub rotation: f64,
    /// Velocità del pan (per Pan).
    pub velocity_x: f64,
    pub velocity_y: f64,
    /// Centro del gesture.
    pub center_x: f64,
    pub center_y: f64,
}

impl GestureInfo {
    /// Crea info per un pinch gesture.
    pub fn pinch(scale: f64, center_x: f64, center_y: f64) -> Self {
        Self {
            gesture_type: GestureType::Pinch,
            scale,
            rotation: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            center_x,
            center_y,
        }
    }

    /// Crea info per un rotate gesture.
    pub fn rotate(rotation: f64, center_x: f64, center_y: f64) -> Self {
        Self {
            gesture_type: GestureType::Rotate,
            scale: 1.0,
            rotation,
            velocity_x: 0.0,
            velocity_y: 0.0,
            center_x,
            center_y,
        }
    }
}

/// Stato del drag in corso.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragState {
    /// Drag non attivo.
    None,
    /// Drag iniziato.
    Started,
    /// Drag in corso.
    Moving,
    /// Drag terminato.
    Ended,
}

/// Informazioni su un drag operation.
#[derive(Debug, Clone)]
pub struct DragInfo {
    /// Stato corrente del drag.
    pub state: DragState,
    /// Posizione X corrente.
    pub x: i32,
    /// Posizione Y corrente.
    pub y: i32,
    /// Offset dall'inizio del drag.
    pub delta_x: i32,
    pub delta_y: i32,
    /// Bottone del mouse che ha iniziato il drag.
    pub button: MouseButton,
}

impl DragInfo {
    /// Crea info per un nuovo drag.
    pub fn start(x: i32, y: i32, button: MouseButton) -> Self {
        Self {
            state: DragState::Started,
            x,
            y,
            delta_x: 0,
            delta_y: 0,
            button,
        }
    }

    /// Aggiorna la posizione del drag.
    pub fn update(&mut self, x: i32, y: i32) {
        self.delta_x = x - self.x;
        self.delta_y = y - self.y;
        self.x = x;
        self.y = y;
        self.state = DragState::Moving;
    }

    /// Termina il drag.
    pub fn end(&mut self) {
        self.state = DragState::Ended;
    }
}

/// Bottone del mouse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseButton {
    /// Bottone sinistro.
    Left,
    /// Bottone destro.
    Right,
    /// Bottone centrale (rotella).
    Middle,
    /// Nessun bottone.
    None,
}

/// Eventi di sistema unificati.
#[derive(Debug, Clone)]
pub enum SystemEvent {
    // ── Hover Events ─────────────────────────────────────────────────────────────
    /// Mouse è entrato nel widget.
    HoverEnter {
        /// ID del widget.
        widget: WidgetId,
        /// Posizione X del mouse nelle coordinate del widget.
        x: i32,
        /// Posizione Y del mouse nelle coordinate del widget.
        y: i32,
    },

    /// Mouse è uscito dal widget.
    HoverExit {
        /// ID del widget.
        widget: WidgetId,
    },

    /// Mouse si sta muovendo dentro il widget (solo se tracking attivo).
    HoverMove {
        /// ID del widget.
        widget: WidgetId,
        /// Posizione X del mouse.
        x: i32,
        /// Posizione Y del mouse.
        y: i32,
    },

    /// Hover prolungato - timer scaduto (per mostrare tooltip).
    HoverTimer {
        /// ID del widget.
        widget: WidgetId,
        /// Durata dell'hover in millisecondi.
        duration_ms: u32,
    },

    // ── Click Events ────────────────────────────────────────────────────────────
    /// Doppio click.
    DoubleClick {
        /// ID del widget.
        widget: WidgetId,
        /// Posizione X.
        x: i32,
        /// Posizione Y.
        y: i32,
        /// Bottone cliccato.
        button: MouseButton,
    },

    /// Click destro.
    RightClick {
        /// ID del widget.
        widget: WidgetId,
        /// Posizione X.
        x: i32,
        /// Posizione Y.
        y: i32,
    },

    /// Click centrale (rotella).
    MiddleClick {
        /// ID del widget.
        widget: WidgetId,
        /// Posizione X.
        x: i32,
        /// Posizione Y.
        y: i32,
    },

    // ── Wheel Events ────────────────────────────────────────────────────────────
    /// Rotella mouse (scroll).
    Wheel {
        /// ID del widget.
        widget: WidgetId,
        /// Delta verticale (positivu = scroll up).
        delta_y: i16,
        /// Delta orizzontale.
        delta_x: i16,
        /// Posizione del mouse.
        mouse_x: i32,
        mouse_y: i32,
    },

    // ── Drag Events ──────────────────────────────────────────────────────────────
    /// Inizio drag.
    DragStart {
        /// ID del widget.
        widget: WidgetId,
        /// Posizione iniziale.
        x: i32,
        y: i32,
        /// Bottone del mouse.
        button: MouseButton,
    },

    /// Drag in corso.
    DragMove {
        /// ID del widget.
        widget: WidgetId,
        /// Posizione corrente.
        x: i32,
        y: i32,
        /// Offset dall'inizio.
        delta_x: i32,
        delta_y: i32,
    },

    /// Fine drag.
    DragEnd {
        /// ID del widget.
        widget: WidgetId,
        /// Posizione finale.
        x: i32,
        y: i32,
        /// Bottone del mouse.
        button: MouseButton,
    },

    // ── Touch Events ────────────────────────────────────────────────────────────
    /// Touch iniziato.
    TouchStart {
        /// ID del widget.
        widget: WidgetId,
        /// Punti di touch.
        touches: Vec<TouchPoint>,
    },

    /// Touch in movimento.
    TouchMove {
        /// ID del widget.
        widget: WidgetId,
        /// Punti di touch.
        touches: Vec<TouchPoint>,
    },

    /// Touch terminato.
    TouchEnd {
        /// ID del widget.
        widget: WidgetId,
        /// Punti di touch rimanenti.
        touches: Vec<TouchPoint>,
    },

    // ── Gesture Events ─────────────────────────────────────────────────────────
    /// Gesture riconosciuta.
    Gesture {
        /// ID del widget.
        widget: WidgetId,
        /// Info sul gesture.
        gesture: GestureInfo,
    },
}

/// Trait per gestire gli eventi hover.
/// Implementa questo trait per ricevere eventi hover su un widget.
pub trait HoverHandler {
    /// Chiamato quando il mouse entra nel widget.
    fn on_hover_enter(&mut self, _widget: WidgetId, _x: i32, _y: i32) {}

    /// Chiamato quando il mouse esce dal widget.
    fn on_hover_exit(&mut self, _widget: WidgetId) {}

    /// Chiamato quando il mouse si muove dentro il widget.
    fn on_hover_move(&mut self, _widget: WidgetId, _x: i32, _y: i32) {}

    /// Chiamato quando l'hover timer scatta (hover prolungato).
    fn on_hover_timer(&mut self, _widget: WidgetId, _duration_ms: u32) {}
}

/// Trait per gestire gli eventi click avanzati.
pub trait ClickHandler {
    /// Chiamato su doppio click.
    fn on_double_click(&mut self, _widget: WidgetId, _x: i32, _y: i32, _button: MouseButton) {}

    /// Chiamato su click destro.
    fn on_right_click(&mut self, _widget: WidgetId, _x: i32, _y: i32) {}

    /// Chiamato su click centrale.
    fn on_middle_click(&mut self, _widget: WidgetId, _x: i32, _y: i32) {}
}

/// Trait per gestire la rotella mouse.
pub trait WheelHandler {
    /// Chiamato quando la rotella viene girata.
    fn on_wheel(
        &mut self,
        _widget: WidgetId,
        _delta_y: i16,
        _delta_x: i16,
        _mouse_x: i32,
        _mouse_y: i32,
    ) {
    }
}

/// Trait per gestire drag & drop.
pub trait DragHandler {
    /// Chiamato quando inizia il drag.
    fn on_drag_start(&mut self, _widget: WidgetId, _x: i32, _y: i32, _button: MouseButton) {}

    /// Chiamato durante il drag.
    fn on_drag_move(&mut self, _widget: WidgetId, _x: i32, _y: i32, _delta_x: i32, _delta_y: i32) {}

    /// Chiamato quando finisce il drag.
    fn on_drag_end(&mut self, _widget: WidgetId, _x: i32, _y: i32, _button: MouseButton) {}
}

/// Configurazione per il tracking degli eventi hover.
#[derive(Debug, Clone)]
pub struct HoverConfig {
    /// Abilita tracking hover_enter/hover_exit.
    pub track_enter_exit: bool,
    /// Abilita tracking hover_move (può essere costoso).
    pub track_move: bool,
    /// Timeout in ms per l'hover timer (0 = disabilitato).
    pub hover_timer_ms: u32,
}

impl Default for HoverConfig {
    fn default() -> Self {
        Self {
            track_enter_exit: true,
            track_move: false,
            hover_timer_ms: 0,
        }
    }
}

impl HoverConfig {
    /// Crea una configurazione standard.
    pub fn new() -> Self {
        Self::default()
    }

    /// Abilita solo enter/exit.
    pub fn enter_exit_only() -> Self {
        Self {
            track_enter_exit: true,
            track_move: false,
            hover_timer_ms: 0,
        }
    }

    /// Abilita hover timer con timeout specificato.
    pub fn with_hover_timer(mut self, timeout_ms: u32) -> Self {
        self.hover_timer_ms = timeout_ms;
        self
    }

    /// Abilita tracking del movimento.
    pub fn with_move_tracking(mut self) -> Self {
        self.track_move = true;
        self
    }
}
