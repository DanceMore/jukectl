use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsData {
    pub any: Vec<String>,
    pub not: Vec<String>,
}
