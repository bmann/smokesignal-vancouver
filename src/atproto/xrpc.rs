use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleError {
    pub error: Option<String>,
    pub error_description: Option<String>,
    pub message: Option<String>,
}

impl SimpleError {
    pub fn error_message(&self) -> String {
        [&self.error, &self.error_description, &self.message]
            .iter()
            .filter_map(|v| (*v).clone())
            .collect::<Vec<String>>()
            .join(": ")
    }
}
