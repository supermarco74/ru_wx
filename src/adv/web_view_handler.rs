//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! WebView protocol handler (`wxWebViewHandler`).

/// Callback that maps a requested URL to the response body.
type UrlHandler = Box<dyn Fn(&str) -> String + Send + Sync>;

/// Custom URL scheme handler for [`crate::WebView`] (`wxWebViewHandler`).
pub struct WebViewHandler {
    scheme: String,
    handler: Option<UrlHandler>,
}

impl WebViewHandler {
    pub fn new(scheme: &str) -> Self {
        Self {
            scheme: scheme.to_string(),
            handler: None,
        }
    }

    pub fn set_handler<F>(&mut self, f: F)
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.handler = Some(Box::new(f));
    }

    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    pub fn handle(&self, url: &str) -> Option<String> {
        let prefix = format!("{}://", self.scheme);
        if !url.starts_with(&prefix) {
            return None;
        }
        let path = &url[prefix.len()..];
        self.handler.as_ref().map(|h| h(path))
    }
}

impl std::fmt::Debug for WebViewHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebViewHandler")
            .field("scheme", &self.scheme)
            .field("handler", &self.handler.is_some())
            .finish()
    }
}
