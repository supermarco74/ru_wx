//! Layout constructors for the WinUI3/XAML bridge.
//!
//! | Module         | Widget           | Reactor equivalent      |
//! |----------------|------------------|-------------------------|
//! | `panel`        | `XamlPanel`      | `reactor::vstack`       |
//! | `sizer`        | `XamlBoxSizer`   | `reactor::vstack/hstack`|
//! | `tab`          | `XamlTab`        | `reactor::TabView`      |
//! | `splitter`     | `XamlSplitter`   | `reactor::SplitView`    |
//! | `scroll`       | `XamlScrolledWindow`| `reactor::ScrollViewer`|

pub mod panel;
pub mod scroll;
pub mod sizer;
pub mod splitter;
pub mod tab;

pub use panel::XamlPanel;
pub use scroll::XamlScrolledWindow;
pub use sizer::{XamlBoxSizer, XamlOrientation};
pub use splitter::XamlSplitter;
pub use tab::XamlTab;
