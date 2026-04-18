//! SEO check domains. Each module owns a slice of the analysis surface.
//!
//! Add a new module here when introducing a new domain; add a new function
//! to an existing module when growing one. Each module's `run(&Ctx)`
//! returns its own `Vec<Check>`.

pub mod title;
pub mod meta;
pub mod content;
pub mod slug;
pub mod images;
pub mod keywords;
pub mod links;
pub mod readability;
