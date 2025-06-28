use serde::{Deserialize, Serialize};

#[derive(Deserialize, Default)]
pub struct TabSelector {
    pub tab: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct TabLink {
    pub name: String,
    pub label: String,
    pub url: String,
    pub active: bool,
}
