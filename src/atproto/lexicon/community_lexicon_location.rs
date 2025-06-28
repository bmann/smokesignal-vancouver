use serde::{Deserialize, Serialize};

pub const NSID: &str = "community.lexicon.location";

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "$type")]
pub enum Address {
    #[serde(rename = "community.lexicon.location.address")]
    Current {
        country: String,

        #[serde(
            rename = "postalCode",
            skip_serializing_if = "Option::is_none",
            default
        )]
        postal_code: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none", default)]
        region: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none", default)]
        locality: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none", default)]
        street: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none", default)]
        name: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "$type")]
pub enum Geo {
    #[serde(rename = "community.lexicon.location.geo")]
    Current {
        latitude: String,

        longitude: String,

        #[serde(skip_serializing_if = "Option::is_none", default)]
        name: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "$type")]
pub enum Fsq {
    #[serde(rename = "community.lexicon.location.fsq")]
    Current {
        fsq_place_id: String,

        #[serde(skip_serializing_if = "Option::is_none", default)]
        name: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "$type")]
pub enum Hthree {
    #[serde(rename = "community.lexicon.location.hthree")]
    Current {
        value: String,

        #[serde(skip_serializing_if = "Option::is_none", default)]
        name: Option<String>,
    },
}
