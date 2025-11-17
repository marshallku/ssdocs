use blake3;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

// Define characters that should NOT be percent-encoded
// https://url.spec.whatwg.org/#path-percent-encode-set
const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
const PATH: &AsciiSet = &FRAGMENT.add(b'#').add(b'?').add(b'{').add(b'}');

/// Encode a slug or tag for use in URLs and filesystem paths
/// - Percent-encodes non-ASCII characters
/// - Keeps ASCII letters, numbers, hyphens, underscores, and dots as-is
/// - Truncates very long strings with a hash for uniqueness
pub fn encode_for_url(input: &str) -> String {
    let encoded = utf8_percent_encode(input, PATH).to_string();

    // Filesystem limit is usually 255 bytes, keep some margin
    const MAX_LEN: usize = 200;
    if encoded.len() > MAX_LEN {
        let hash = blake3::hash(encoded.as_bytes());
        format!("{}-{}", &encoded[..180], &hash.to_hex()[..16])
    } else {
        encoded
    }
}

/// Decode a percent-encoded slug or tag back to the original string
pub fn decode_from_url(input: &str) -> String {
    percent_encoding::percent_decode_str(input)
        .decode_utf8()
        .unwrap_or_else(|_| std::borrow::Cow::Borrowed(input))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_ascii() {
        assert_eq!(encode_for_url("hello-world"), "hello-world");
        assert_eq!(encode_for_url("test_file.md"), "test_file.md");
    }

    #[test]
    fn test_encode_korean() {
        let encoded = encode_for_url("한글-테스트");
        assert!(encoded.contains("%ED%95%9C"));
    }

    #[test]
    fn test_decode() {
        let encoded = encode_for_url("한글-테스트");
        let decoded = decode_from_url(&encoded);
        assert_eq!(decoded, "한글-테스트");
    }

    #[test]
    fn test_encode_long_string() {
        let long_string = "가".repeat(100); // 100 Korean characters
        let encoded = encode_for_url(&long_string);
        assert!(encoded.len() <= 200);
        assert!(encoded.contains('-')); // Should have hash separator
    }
}
