use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::encoding_errors::EncodingError;

pub trait ToBase64 {
    fn to_base64(&self) -> Result<Cow<str>>;
}

impl<T: Serialize> ToBase64 for T {
    fn to_base64(&self) -> Result<Cow<str>> {
        let json_bytes =
            serde_json::to_vec(&self).map_err(EncodingError::JsonSerializationFailed)?;
        let encoded_json_bytes = general_purpose::URL_SAFE_NO_PAD.encode(json_bytes);
        Ok(Cow::Owned(encoded_json_bytes))
    }
}

pub trait FromBase64: Sized {
    fn from_base64<Input: ?Sized + AsRef<[u8]>>(raw: &Input) -> Result<Self>;
}

impl<T: for<'de> Deserialize<'de> + Sized> FromBase64 for T {
    fn from_base64<Input: ?Sized + AsRef<[u8]>>(raw: &Input) -> Result<Self> {
        let content = general_purpose::URL_SAFE_NO_PAD
            .decode(raw)
            .map_err(EncodingError::Base64DecodingFailed)?;
        serde_json::from_slice(&content)
            .map_err(|err| EncodingError::JsonDeserializationFailed(err).into())
    }
}
