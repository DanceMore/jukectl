extern crate serde;
use serde::Deserialize;
use serde::Serialize;
use serde_json;
use std::fmt;

// TagData, useful holder for any_tags vs not_tags
#[derive(Serialize, Deserialize)]
pub struct TagsData {
    pub any: Vec<String>,
    pub not: Vec<String>,
}

// Implement the Debug trait for TagsData
impl<> fmt::Debug for TagsData<> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\n    any: {:?},\n    not: {:?}\n}}", self.any, self.not)
    }
}

impl<> TagsData<> {
    pub fn to_json(&self) -> String {
        match serde_json::to_string(self) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("[!!!] Error serializing TagsData to JSON: {}", err);
                std::process::exit(1); // Exit with a non-zero status code
            }
        }
    }
}

pub fn parse_tags_data_from_argv(tags: &str, not_tags: &str) -> TagsData {
    TagsData {
        any: tags.split(',').map(|s| s.trim().to_string()).collect(),
        not: not_tags.split(',').map(|s| s.trim().to_string()).collect(),
    }
}
