use std::fmt::Write as _;

use anyhow::{Result, bail};
use quick_xml::Reader;
use quick_xml::events::Event;
use sha2::{Digest, Sha256};

pub fn normalize_xml(value: &str) -> String {
    value
        .strip_prefix('\u{feff}')
        .unwrap_or(value)
        .replace("\r\n", "\n")
        .trim()
        .to_owned()
}

pub fn xml_sha256(value: &str) -> String {
    let normalized = normalize_xml(value);
    let digest = Sha256::digest(normalized.as_bytes());

    let mut result = String::with_capacity(digest.len() * 2);

    for byte in digest {
        write!(&mut result, "{byte:02x}").expect("writing to String cannot fail");
    }

    result
}

/// appScreener DataFlow XML содержит несколько верхнеуровневых секций,
/// например `condition` и `taintFlowChain`.
///
/// Поэтому для проверки добавляется временный корневой элемент.
/// В appScreener он не отправляется.
pub fn validate_xml_fragment(xml: &str) -> Result<()> {
    let wrapped = format!(
        "<appscreener-fragment>\
         {xml}\
         </appscreener-fragment>"
    );

    let mut reader = Reader::from_str(&wrapped);

    reader.config_mut().trim_text(false);

    loop {
        match reader.read_event() {
            Ok(Event::Eof) => return Ok(()),

            // DTD для паттернов не нужен и создаёт лишнюю поверхность атак.
            Ok(Event::DocType(_)) => {
                bail!("DOCTYPE declarations are not allowed");
            }

            // XML declaration внутри добавленного root некорректна.
            Ok(Event::Decl(_)) => {
                bail!("XML declaration is not allowed in a pattern fragment");
            }

            Ok(_) => {}

            Err(error) => {
                bail!("{error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_dataflow_fragment_with_two_roots() {
        let xml = r#"
<condition>
    <callExpr/>
</condition>
<taintFlowChain>
    <taintFlow/>
</taintFlowChain>
"#;

        validate_xml_fragment(xml).unwrap();
    }

    #[test]
    fn rejects_unclosed_element() {
        let xml = "<condition><callExpr></condition>";

        assert!(validate_xml_fragment(xml).is_err());
    }

    #[test]
    fn normalizes_bom_and_line_endings() {
        let xml = "\u{feff}<condition/>\r\n";

        assert_eq!(normalize_xml(xml), "<condition/>");
    }

    #[test]
    fn hash_ignores_outer_whitespace_and_crlf() {
        assert_eq!(
            xml_sha256("<condition/>\r\n"),
            xml_sha256("  <condition/>\n")
        );
    }
}
