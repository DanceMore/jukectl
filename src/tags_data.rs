use serde::Serialize;
use serde::Deserialize;

#[derive(Serialize, Deserialize)]
pub struct TagsData {
    pub any: Vec<String>,
    pub not: Vec<String>,
}
