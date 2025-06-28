use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("error-xrpc-client-1 Malformed PutRecord response: {0:?}")]
    PutRecordResponseFailure(reqwest::Error),

    #[error("error-xrpc-client-2 Malformed CreateRecord response: {0:?}")]
    CreateRecordResponseFailure(reqwest::Error),

    #[error("error-xrpc-client-3 XRPC error from server: {0}")]
    ServerError(String),

    #[error("error-xrpc-client-4 Invalid record format: {0}")]
    InvalidRecordFormat(String),
}

#[derive(Debug, Error)]
pub enum UriError {
    #[error("error-uri-1 Invalid AT-URI: repository missing")]
    RepositoryMissing,

    #[error("error-uri-2 Invalid AT-URI: collection missing")]
    CollectionMissing,

    #[error("error-uri-3 Invalid AT-URI: rkey missing")]
    RkeyMissing,

    #[error("error-uri-4 Invalid AT-URI")]
    InvalidFormat,

    #[error("error-uri-5 Invalid AT-URI: repository contains invalid characters")]
    InvalidRepository,

    #[error("error-uri-6 Invalid AT-URI: collection contains invalid characters")]
    InvalidCollection,

    #[error("error-uri-7 Invalid AT-URI: rkey contains invalid characters")]
    InvalidRkey,

    #[error("error-uri-8 Invalid AT-URI: path traversal attempt detected")]
    PathTraversalAttempt,

    #[error("error-uri-9 Invalid AT-URI: repository too long (max 253 chars)")]
    RepositoryTooLong,

    #[error("error-uri-10 Invalid AT-URI: collection too long (max 128 chars)")]
    CollectionTooLong,

    #[error("error-uri-11 Invalid AT-URI: rkey too long (max 512 chars)")]
    RkeyTooLong,
}
