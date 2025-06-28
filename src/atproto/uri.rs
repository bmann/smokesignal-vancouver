use anyhow::Result;

use crate::{atproto::errors::UriError, validation::is_valid_hostname};

// Constants for maximum lengths
const MAX_REPOSITORY_LENGTH: usize = 253; // DNS name length limit
const MAX_COLLECTION_LENGTH: usize = 128;
const MAX_RKEY_LENGTH: usize = 512;

/// Validates a repository name for AT Protocol URIs
///
/// Repository names should generally follow host name rules:
/// - Alphanumeric characters, hyphens, and periods
/// - No consecutive periods
/// - Cannot start or end with period or hyphen
fn is_valid_repository(repository: &str) -> bool {
    if repository.is_empty() || repository.len() > MAX_REPOSITORY_LENGTH {
        return false;
    }

    // Check for invalid characters
    if !repository
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == ':')
    {
        return false;
    }

    // TODO: If starts with "did:plc:" then validate encoded string and length
    if repository.starts_with("did:plc:") {
        return true;
    }

    // TODO: If starts with "did:web:" then validate hostname and parts
    if repository.starts_with("did:web:") {
        return true;
    }

    is_valid_hostname(repository)
}

/// Validates a collection name for AT Protocol URIs
///
/// Collections should follow namespace-like naming:
/// - Alphanumeric characters, hyphens, underscores, and periods
/// - No path traversal sequences
fn is_valid_collection(collection: &str) -> bool {
    if collection.is_empty() || collection.len() > MAX_COLLECTION_LENGTH {
        return false;
    }

    // Check for invalid characters
    if !collection
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return false;
    }

    // Check for path traversal attempts
    if collection.contains("../") || collection == ".." {
        return false;
    }

    true
}

/// Validates a record key (rkey) for AT Protocol URIs
///
/// Record keys have more flexible rules but shouldn't contain characters
/// that could cause problems in URLs or filesystems
fn is_valid_rkey(rkey: &str) -> bool {
    if rkey.is_empty() || rkey.len() > MAX_RKEY_LENGTH {
        return false;
    }

    // Block characters that are particularly problematic
    if rkey.chars().any(|c| {
        c == '<'
            || c == '>'
            || c == '"'
            || c == '\''
            || c == '`'
            || c == '\\'
            || c == '|'
            || c == '*'
            || c == '?'
            || c == '#'
    }) {
        return false;
    }

    // Check for path traversal attempts
    if rkey.contains("../") || rkey == ".." {
        return false;
    }

    true
}

/// Parses and validates an AT Protocol URI string into its components
///
/// AT Protocol URIs follow the format: at://repository/collection/rkey
/// This function validates each component for proper format and security
pub fn parse_aturi(uri: &str) -> Result<(String, String, String)> {
    // Validate URI has the correct prefix
    if !uri.starts_with("at://") {
        return Err(UriError::InvalidFormat.into());
    }

    let value = uri.strip_prefix("at://").unwrap(); // Safe because we checked above

    // Split the URI into components
    let mut components = value.split('/');

    // Extract repository
    let repository = components.next().ok_or(UriError::RepositoryMissing)?;

    // Validate repository
    if repository.len() > MAX_REPOSITORY_LENGTH {
        return Err(UriError::RepositoryTooLong.into());
    }
    if !is_valid_repository(repository) {
        return Err(UriError::InvalidRepository.into());
    }

    // Extract collection
    let collection = components.next().ok_or(UriError::CollectionMissing)?;

    // Validate collection
    if collection.len() > MAX_COLLECTION_LENGTH {
        return Err(UriError::CollectionTooLong.into());
    }
    if !is_valid_collection(collection) {
        return Err(UriError::InvalidCollection.into());
    }

    // Extract record key
    let rkey = components.next().ok_or(UriError::RkeyMissing)?;

    // Validate record key
    if rkey.len() > MAX_RKEY_LENGTH {
        return Err(UriError::RkeyTooLong.into());
    }
    if !is_valid_rkey(rkey) {
        return Err(UriError::InvalidRkey.into());
    }

    // Check for any path traversal attempts
    if repository.contains("..") || collection.contains("..") || rkey.contains("..") {
        return Err(UriError::PathTraversalAttempt.into());
    }

    // Return validated components
    Ok((
        repository.to_string(),
        collection.to_string(),
        rkey.to_string(),
    ))
}
