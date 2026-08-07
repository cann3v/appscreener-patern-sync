mod loader;
mod xml;

pub use loader::{LocalPattern, load_local_patterns};

pub use xml::{normalize_xml, xml_sha256};
