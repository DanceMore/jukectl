use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
pub struct TagsData {
    pub any: Vec<String>,
    pub not: Vec<String>,
}
