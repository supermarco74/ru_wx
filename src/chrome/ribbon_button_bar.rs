//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Ribbon button bar (`wxRibbonButtonBar`).

/// Image position relative to label on a ribbon button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RibbonImagePosition {
    /// Image to the left of the label (default for ribbon buttons).
    #[default]
    ImageLeading,
    /// Image above the label.
    ImageAbove,
    /// Image below the label.
    ImageBelow,
    /// Image to the right of the label.
    ImageTrailing,
    /// Image only, no label.
    ImageOnly,
}

impl RibbonImagePosition {
    /// Convert to AppKit NSCellImagePosition.
    #[cfg(target_os = "macos")]
    pub fn to_ns_position(self) -> objc2_app_kit::NSCellImagePosition {
        match self {
            RibbonImagePosition::ImageAbove => objc2_app_kit::NSCellImagePosition::ImageAbove,
            RibbonImagePosition::ImageBelow => objc2_app_kit::NSCellImagePosition::ImageBelow,
            RibbonImagePosition::ImageLeading => objc2_app_kit::NSCellImagePosition::ImageLeading,
            RibbonImagePosition::ImageTrailing => objc2_app_kit::NSCellImagePosition::ImageTrailing,
            RibbonImagePosition::ImageOnly => objc2_app_kit::NSCellImagePosition::ImageOnly,
        }
    }
}

/// Tipo di controllo ribbon (per Microsoft Fluent UX).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RibbonControlType {
    /// Regular push button (es. "Incolla").
    Button,
    /// Toggle button with on/off state (es. "Grassetto").
    ToggleButton,
    /// Split button with dropdown arrow (es. "Nuova diapositiva").
    SplitButton,
}

/// RGBA image data for a ribbon button icon.
pub type RgbaImage = (u32, u32, Vec<u8>);

/// Data for a single button: id, label, optional RGBA icon image.
#[derive(Debug, Clone)]
pub struct RibbonButtonData {
    pub id: u16,
    pub label: String,
    /// RGBA image data (width, height, bytes) for the button icon.
    pub image_data: Option<RgbaImage>,
    /// RGBA image data for hover state (image shown when mouse is over button).
    pub hover_image_data: Option<RgbaImage>,
    /// Position of image relative to label.
    pub image_position: RibbonImagePosition,
    /// Keytip for keyboard access (es. "B" for Bold).
    pub keytip: Option<String>,
    /// Whether the button is enabled.
    pub is_enabled: bool,
    /// Control type (Button, ToggleButton, SplitButton).
    pub control_type: RibbonControlType,
    /// For ToggleButton: current toggle state.
    pub is_toggled: bool,
}

impl RibbonButtonData {
    pub fn new(id: u16, label: &str) -> Self {
        Self {
            id,
            label: label.to_string(),
            image_data: None,
            hover_image_data: None,
            image_position: RibbonImagePosition::default(),
            keytip: None,
            is_enabled: true,
            control_type: RibbonControlType::Button,
            is_toggled: false,
        }
    }

    /// Create a button with an RGBA icon image.
    pub fn with_icon(id: u16, label: &str, image: RgbaImage) -> Self {
        Self {
            id,
            label: label.to_string(),
            image_data: Some(image),
            hover_image_data: None,
            image_position: RibbonImagePosition::default(),
            keytip: None,
            is_enabled: true,
            control_type: RibbonControlType::Button,
            is_toggled: false,
        }
    }

    /// Create a button with an RGBA icon image and hover image.
    pub fn with_icon_and_hover(
        id: u16,
        label: &str,
        image: RgbaImage,
        hover_image: RgbaImage,
    ) -> Self {
        Self {
            id,
            label: label.to_string(),
            image_data: Some(image),
            hover_image_data: Some(hover_image),
            image_position: RibbonImagePosition::default(),
            keytip: None,
            is_enabled: true,
            control_type: RibbonControlType::Button,
            is_toggled: false,
        }
    }

    /// Create a button with a solid color icon (for demo purposes).
    /// The icon is a colored square with the given RGB color.
    pub fn with_color_icon(id: u16, label: &str, r: u8, g: u8, b: u8, size: u32) -> Self {
        let pixel = [r, g, b, 255];
        let pixels: Vec<u8> = pixel
            .into_iter()
            .cycle()
            .take((size * size * 4) as usize)
            .collect();
        Self::with_icon(id, label, (size, size, pixels))
    }

    /// Create a toggle button (es. Bold, Italic).
    pub fn toggle(id: u16, label: &str, image: RgbaImage) -> Self {
        Self {
            id,
            label: label.to_string(),
            image_data: Some(image),
            hover_image_data: None,
            image_position: RibbonImagePosition::default(),
            keytip: None,
            is_enabled: true,
            control_type: RibbonControlType::ToggleButton,
            is_toggled: false,
        }
    }

    /// Create a split button with dropdown.
    pub fn split(id: u16, label: &str, image: RgbaImage) -> Self {
        Self {
            id,
            label: label.to_string(),
            image_data: Some(image),
            hover_image_data: None,
            image_position: RibbonImagePosition::default(),
            keytip: None,
            is_enabled: true,
            control_type: RibbonControlType::SplitButton,
            is_toggled: false,
        }
    }

    /// Set the image position relative to the label.
    pub fn with_image_position(&mut self, pos: RibbonImagePosition) -> &mut Self {
        self.image_position = pos;
        self
    }

    /// Set the keytip for keyboard access.
    pub fn with_keytip(mut self, keytip: &str) -> Self {
        self.keytip = Some(keytip.to_string());
        self
    }
}

/// Horizontal strip of ribbon tools (`wxRibbonButtonBar`).
#[derive(Debug, Clone, Default)]
pub struct RibbonButtonBar {
    buttons: Vec<RibbonButtonData>,
}

impl RibbonButtonBar {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a text-only button (no icon).
    pub fn add_button(&mut self, id: u16, label: &str) {
        self.buttons.push(RibbonButtonData::new(id, label));
    }

    /// Add a button with a solid color icon (24x24 pixels).
    pub fn add_button_with_color(&mut self, id: u16, label: &str, r: u8, g: u8, b: u8) {
        self.buttons
            .push(RibbonButtonData::with_color_icon(id, label, r, g, b, 24));
    }

    /// Add a button with a solid color icon and specific image position.
    pub fn add_button_with_color_and_position(
        &mut self,
        id: u16,
        label: &str,
        r: u8,
        g: u8,
        b: u8,
        position: RibbonImagePosition,
    ) {
        let mut btn = RibbonButtonData::with_color_icon(id, label, r, g, b, 24);
        btn.image_position = position;
        self.buttons.push(btn);
    }

    /// Add a button with an RGBA icon image and specific position.
    pub fn add_button_with_icon_and_position(
        &mut self,
        id: u16,
        label: &str,
        rgba: (u32, u32, Vec<u8>),
        position: RibbonImagePosition,
    ) {
        let mut btn = RibbonButtonData::with_icon(id, label, rgba);
        btn.image_position = position;
        self.buttons.push(btn);
    }

    /// Add a button with an RGBA icon image, hover image, and specific position.
    pub fn add_button_with_icon_and_hover_and_position(
        &mut self,
        id: u16,
        label: &str,
        rgba: (u32, u32, Vec<u8>),
        hover_rgba: (u32, u32, Vec<u8>),
        position: RibbonImagePosition,
    ) {
        let mut btn = RibbonButtonData::with_icon_and_hover(id, label, rgba, hover_rgba);
        btn.image_position = position;
        self.buttons.push(btn);
    }

    pub fn button_count(&self) -> usize {
        self.buttons.len()
    }

    /// Returns all button data.
    pub fn buttons(&self) -> Vec<&RibbonButtonData> {
        self.buttons.iter().collect()
    }
}
