//! Smoke Signal Errors
//!
//! All errors are represented as a string in this format:
//!
//! "error-<domain>-<number> <message>: <details>"
//!
//! The first part containing the "error-" prefix, domain, and number, is used
//! to uniquely identify the error. This standard code format is used to convey
//! the source (easy for humans and agents to search for and reference) and
//! localize.
//!
//! The "message" part contains a simple message in English explaining the
//! error. When an error does not have a localized string, this message may be
//! displayed to the user or client. The message cannot contain a ":" character
//! because it is used to seperate the message from the error details.
//!
//! The last "details" part contains additional error details that is useful
//! for logging or debugging.
//!
//! Errors are defined in source files that have the "_errors" suffix or in
//! sub-packages named "errors".
//!
use std::string::ToString;

pub fn expand_error<S: ToString>(err: S) -> (String, String) {
    let err: String = err.to_string();
    if !err.starts_with("error-") {
        panic!("incorrect error format: {err}")
    }
    let (error_code, extra) = match err.split_once(' ') {
        Some((error_code, extra)) => (error_code, extra),
        _ => return (err.to_string(), "".to_string()),
    };

    match extra.split_once(':') {
        Some((message, _)) => (error_code.to_string(), message.to_string()),
        None => (error_code.to_string(), extra.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_error() {
        assert_eq!(
            expand_error("error-example-1"),
            ("error-example-1".to_string(), "".to_string())
        );
        assert_eq!(
            expand_error("error-example-1 An example"),
            ("error-example-1".to_string(), "An example".to_string())
        );
        assert_eq!(
            expand_error("error-example-1 An example: With details"),
            ("error-example-1".to_string(), "An example".to_string())
        );
    }
}
