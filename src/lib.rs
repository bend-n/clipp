//! simple possibly cross platform clipboard crate
//!
//! ```
//! clipp::copy("wow such clipboard");
//! assert_eq!(clipp::paste(), "wow such clipboard");
//! ```
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
mod providers;

use std::{fmt::Display, sync::OnceLock};

static CLIP: OnceLock<providers::Board> = OnceLock::new();

/// Copy text to the clipboard.
pub fn copy(text: impl Display) {
    CLIP.get_or_init(providers::provide).0(&format!("{text}"));
}

/// Paste text from the clipboard.
pub fn paste() -> String {
    CLIP.get_or_init(providers::provide).1()
}
