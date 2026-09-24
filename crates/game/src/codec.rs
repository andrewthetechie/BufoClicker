//! The existing export/import codec: `btoa(encodeURIComponent(JSON))`.
//!
//! Pure Rust (byte-identical to the browser's `btoa`/`encodeURIComponent`) so
//! the codec is natively testable. The browser crate uses these same functions.

use base64::Engine;

/// `encodeURIComponent`: escape everything except `A-Z a-z 0-9 - _ . ! ~ * ' ( )`,
/// percent-encoding each UTF-8 byte of anything else.
fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric()
            || matches!(c, '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')')
        {
            out.push(c);
        } else {
            out.push('%');
            out.push_str(&format!("{b:02X}"));
        }
    }
    out
}

/// `decodeURIComponent`: decode `%XX` sequences to UTF-8 bytes.
fn decode_uri_component(s: &str) -> Result<String, String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err("truncated percent-escape".to_string());
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).map_err(|e| e.to_string())?;
            let b = u8::from_str_radix(hex, 16).map_err(|e| e.to_string())?;
            out.push(b);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|e| e.to_string())
}

/// Encode a JSON string as `btoa(encodeURIComponent(json))`.
pub fn encode_export(json: &str) -> String {
    let encoded = encode_uri_component(json);
    base64::engine::general_purpose::STANDARD.encode(encoded.as_bytes())
}

/// Decode a `btoa(encodeURIComponent(json))` string back to the JSON.
pub fn decode_export(encoded: &str) -> Result<String, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|e| e.to_string())?;
    let decoded = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    decode_uri_component(&decoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::{parse_legacy, serialize_v2};

    #[test]
    fn encoded_export_fixture_decodes_and_roundtrips() {
        let encoded = include_str!("../../../tests/fixtures/encoded-export.txt");
        let json = decode_export(encoded).expect("fixture must decode");
        // The fixture is the btoa(encodeURIComponent(JSON)) of `purchased.json`.
        let state = parse_legacy(&json).expect("decoded fixture is a legacy save");
        assert!(state.generator_count("tadpole") > 0.0);
        // Re-encoding round-trips byte-for-byte.
        assert_eq!(decode_export(&encode_export(&json)).unwrap(), json);
    }

    #[test]
    fn v2_export_roundtrips_through_codec() {
        let c = crate::catalog::Catalogs::from_json(
            include_str!("../../../assets/data/generators.json"),
            include_str!("../../../assets/data/upgrades.json"),
            include_str!("../../../assets/data/achievements.json"),
        )
        .unwrap();
        let state = crate::state::GameState::default_state(&c);
        let json = serialize_v2(&state, 1_700_000_000_000.0);
        let encoded = encode_export(&json);
        assert_eq!(decode_export(&encoded).unwrap(), json);
    }

    #[test]
    fn bad_base64_is_an_error() {
        assert!(decode_export("!!!not base64!!!").is_err());
    }
}
