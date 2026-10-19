//! Widget constructors for the WinUI3/XAML bridge.
//!
//! Each widget follows the same pattern:
//! 1. Constructor takes `&XamlFrame` as parent
//! 2. Creates a `WidgetDesc` and appends it to `frame.desc.children`
//! 3. Returns a handle for method chaining and event registration
//!
//! # Available widgets
//!
//! | Module        | Widget        | Reactor equivalent    |
//! |---------------|---------------|-----------------------|
//! | `button`      | `XamlButton`  | `reactor::Button`     |
//! | `text_block`  | `XamlTextBlock`| `reactor::TextBlock` |
//! | `text_box`    | `XamlTextBox` | `reactor::TextBox`    |
//! | `check_box`   | `XamlCheckBox`| `reactor::CheckBox`   |
//! | `radio_button`| `XamlRadioButton`| `reactor::RadioButton`|
//! | `slider`      | `XamlSlider`  | `reactor::Slider`     |
//! | `progress_bar`| `XamlProgressBar`| `reactor::ProgressBar`|
//! | `combo_box`   | `XamlComboBox`| `reactor::ComboBox`   |

pub mod button;
pub mod check_box;
pub mod check_list_box;
pub mod combo_box;
pub mod grid;
pub mod hyperlink;
pub mod image;
pub mod list_box;
pub mod list_ctrl;
pub mod progress_bar;
pub mod radio_button;
pub mod slider;
pub mod text_block;
pub mod text_box;
pub mod tree_view;

pub use button::XamlButton;
pub use check_box::XamlCheckBox;
pub use check_list_box::XamlCheckListBox;
pub use combo_box::XamlComboBox;
pub use grid::XamlGrid;
pub use hyperlink::XamlHyperlink;
pub use image::XamlImage;
pub use list_box::XamlListBox;
pub use list_ctrl::XamlListCtrl;
pub use progress_bar::XamlProgressBar;
pub use radio_button::XamlRadioButton;
pub use slider::XamlSlider;
pub use text_block::XamlTextBlock;
pub use text_box::XamlTextBox;
pub use tree_view::{TreeNodeBuilder, XamlTreeCtrl};
