//! Re-export models from hearth-core so existing `use crate::models::*` imports
//! continue to compile. The types live in `hearth_core::models`.

pub use hearth_core::models::*;
