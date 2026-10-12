//! The asset source.
//!
//! GPUI resolves `svg().path("icons/package.svg")` through this, so the icon set is
//! data on disk rather than paths baked into a binary. `list` exists because the
//! trait requires it; nothing in debris enumerates assets at runtime.

use anyhow::Result;
use gpui::{AssetSource, SharedString};
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

pub struct Assets {
    root: PathBuf,
}

impl Assets {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        // A missing icon must not take the window down: GPUI treats `None` as "no
        // asset", which renders as nothing where a mark would be.
        if path.is_empty() {
            return Ok(None);
        }
        match fs::read(self.root.join(path)) {
            Ok(bytes) => Ok(Some(Cow::Owned(bytes))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let entries = fs::read_dir(self.root.join(path))?;
        Ok(entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .map(SharedString::from)
            .collect())
    }
}
