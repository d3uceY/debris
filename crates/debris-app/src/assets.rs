//! The asset source.
//!
//! GPUI resolves `svg().path("icons/package.svg")` through this, which makes it the one
//! bridge between a path and a pixel. The bytes come from the executable — `build.rs`
//! embedded them — so painting an icon touches no disk and an icon cannot go missing
//! from an installation.

use crate::bundle;
use anyhow::Result;
use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

/// The icon set, as embedded in the binary.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        // A path with no asset in it is `None`, which GPUI paints as nothing. That is
        // the right answer for a mark that is not in the build, and it is not an error.
        Ok(bundle::asset(path).map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(bundle::asset_children(path))
    }
}
