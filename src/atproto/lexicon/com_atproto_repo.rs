use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct StrongRef {
    pub uri: String,
    pub cid: String,
}
