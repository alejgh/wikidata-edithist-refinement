use json_patch::Patch;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Default, Clone, Serialize)]
pub struct WikidataRevision {
    pub id: u64,
    pub parent_id: u64,
    pub timestamp: String,
    pub username: String,
    pub comment: String,
    pub entity_diff: Option<Patch>
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct WikidataItem {
    pub id: u64,
    pub entity_id: String,
    pub entity_json: Value,
    pub revisions: Vec::<WikidataRevision>
}