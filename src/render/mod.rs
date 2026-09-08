//! Renderer trait — see docs/renderer-architecture.md §5.

pub mod table;

use serde_json::Value;
use std::error::Error;

pub struct RenderOptions {
    pub sort: Option<String>,
    pub filter: Option<(String, String)>,
    pub width: Option<usize>,
    pub color: bool,
}

pub trait Renderer {
    fn render(&self, data: &[Value], options: &RenderOptions) -> Result<String, Box<dyn Error>>;
}
