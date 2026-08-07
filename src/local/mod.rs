mod loader;
mod xml;

pub use loader::{LocalPattern, load_local_patterns};

pub(crate) use xml::xml_sha256;
