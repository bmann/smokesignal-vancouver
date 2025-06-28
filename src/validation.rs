//! Validation module that provides utilities for validating hostnames and AT Protocol handles.
//!
//! This module implements RFC-compliant hostname validation and AT Protocol handle formatting rules.

/// Maximum length for a valid hostname as defined in RFC 1035
const MAX_HOSTNAME_LENGTH: usize = 253;

/// Maximum length for a DNS label (component between dots) as defined in RFC 1035
const MAX_LABEL_LENGTH: usize = 63;

/// List of reserved top-level domains that are not valid for AT Protocol handles
const RESERVED_TLDS: [&str; 4] = [".localhost", ".internal", ".arpa", ".local"];

/// Validates if a string is a valid hostname according to RFC standards.
///
/// A valid hostname must:
/// - Only contain alphanumeric characters, hyphens, and periods
/// - Not start or end labels with hyphens
/// - Have labels (parts between dots) with length between 1-63 characters
/// - Have total length not exceeding 253 characters
/// - Not use reserved top-level domains
///
/// # Arguments
/// * `hostname` - The hostname string to validate
///
/// # Returns
/// * `true` if the hostname is valid, `false` otherwise
#[must_use]
pub fn is_valid_hostname(hostname: &str) -> bool {
    // Empty hostnames are invalid
    if hostname.is_empty() || hostname.len() > MAX_HOSTNAME_LENGTH {
        return false;
    }

    // Check if hostname uses any reserved TLDs
    if RESERVED_TLDS.iter().any(|tld| hostname.ends_with(tld)) {
        return false;
    }

    // Ensure all characters are valid hostname characters
    if hostname.bytes().any(|byte| !is_valid_hostname_char(byte)) {
        return false;
    }

    // Validate each DNS label in the hostname
    if hostname.split('.').any(|label| !is_valid_dns_label(label)) {
        return false;
    }

    true
}

/// Checks if a byte is a valid character in a hostname.
///
/// Valid characters are: a-z, A-Z, 0-9, hyphen (-), and period (.)
///
/// # Arguments
/// * `byte` - The byte to check
///
/// # Returns
/// * `true` if the byte is a valid hostname character, `false` otherwise
fn is_valid_hostname_char(byte: u8) -> bool {
    byte.is_ascii_lowercase()
        || byte.is_ascii_uppercase()
        || byte.is_ascii_digit()
        || byte == b'-'
        || byte == b'.'
}

/// Validates if a DNS label is valid according to RFC standards.
///
/// A valid DNS label must:
/// - Not be empty
/// - Not exceed 63 characters
/// - Not start or end with a hyphen
///
/// # Arguments
/// * `label` - The DNS label to validate
///
/// # Returns
/// * `true` if the label is valid, `false` otherwise
fn is_valid_dns_label(label: &str) -> bool {
    !(label.is_empty()
        || label.len() > MAX_LABEL_LENGTH
        || label.starts_with('-')
        || label.ends_with('-'))
}

/// Validates and normalizes an AT Protocol handle.
///
/// A valid AT Protocol handle must:
/// - Be a valid hostname (after stripping any prefixes)
/// - Contain at least one period (.)
/// - Can optionally have "at://" or "@" prefix, which will be removed
///
/// # Arguments
/// * `handle` - The handle string to validate
///
/// # Returns
/// * `Some(String)` containing the normalized handle if valid
/// * `None` if the handle is invalid
#[must_use]
pub fn is_valid_handle(handle: &str) -> Option<String> {
    // Strip optional prefixes to get the core handle
    let trimmed = strip_handle_prefixes(handle);

    // A valid handle must be a valid hostname with at least one period
    if is_valid_hostname(trimmed) && trimmed.contains('.') {
        Some(trimmed.to_string())
    } else {
        None
    }
}

/// Strips common AT Protocol handle prefixes.
///
/// Removes "at://" or "@" prefix if present.
///
/// # Arguments
/// * `handle` - The handle to strip prefixes from
///
/// # Returns
/// * The handle with prefixes removed
fn strip_handle_prefixes(handle: &str) -> &str {
    if let Some(value) = handle.strip_prefix("at://") {
        value
    } else if let Some(value) = handle.strip_prefix('@') {
        value
    } else {
        handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_hostnames() {
        // Valid hostnames
        assert!(is_valid_hostname("example.com"));
        assert!(is_valid_hostname("subdomain.example.com"));
        assert!(is_valid_hostname("with-hyphen.example.com"));
        assert!(is_valid_hostname("123numeric.example.com"));
        assert!(is_valid_hostname("xn--bcher-kva.example.com")); // IDN
    }

    #[test]
    fn test_invalid_hostnames() {
        // Invalid hostnames
        assert!(!is_valid_hostname("")); // Empty
        assert!(!is_valid_hostname("a".repeat(254).as_str())); // Too long
        assert!(!is_valid_hostname("example.localhost")); // Reserved TLD
        assert!(!is_valid_hostname("example.internal")); // Reserved TLD
        assert!(!is_valid_hostname("example.arpa")); // Reserved TLD
        assert!(!is_valid_hostname("example.local")); // Reserved TLD
        assert!(!is_valid_hostname("invalid_char.example.com")); // Invalid underscore
        assert!(!is_valid_hostname("-starts-with-hyphen.example.com")); // Label starts with hyphen
        assert!(!is_valid_hostname("ends-with-hyphen-.example.com")); // Label ends with hyphen
        assert!(!is_valid_hostname(&("a".repeat(64) + ".example.com"))); // Label too long
        assert!(!is_valid_hostname(".starts.with.dot")); // Empty label
        assert!(!is_valid_hostname("ends.with.dot.")); // Empty label
        assert!(!is_valid_hostname("double..dot")); // Empty label
    }

    #[test]
    fn test_valid_handles() {
        // Valid handles
        assert_eq!(
            is_valid_handle("user.example.com"),
            Some("user.example.com".to_string())
        );
        assert_eq!(
            is_valid_handle("at://user.example.com"),
            Some("user.example.com".to_string())
        );
        assert_eq!(
            is_valid_handle("@user.example.com"),
            Some("user.example.com".to_string())
        );
    }

    #[test]
    fn test_invalid_handles() {
        // Invalid handles
        assert_eq!(is_valid_handle("nodots"), None); // No dots
        assert_eq!(is_valid_handle("at://invalid_char.example.com"), None); // Invalid character
        assert_eq!(is_valid_handle("@example.localhost"), None); // Reserved TLD
    }

    #[test]
    fn test_strip_handle_prefixes() {
        assert_eq!(strip_handle_prefixes("example.com"), "example.com");
        assert_eq!(strip_handle_prefixes("at://example.com"), "example.com");
        assert_eq!(strip_handle_prefixes("@example.com"), "example.com");
        // Nested prefixes should only strip the outermost one
        assert_eq!(strip_handle_prefixes("at://@example.com"), "@example.com");
    }
}
