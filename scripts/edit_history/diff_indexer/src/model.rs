use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WikidataRevision {
    pub id: u64,
    pub parent_id: u64,
    pub timestamp: String,
    pub username: String,
    pub comment: String,
    pub entity_diff: Option<Vec::<WikidataOp>>
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WikidataOp {
    pub op: String,
    pub path: String,
    pub value: Option<Value>
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WikidataItem {
    pub id: u64,
    pub entity_id: String,
    pub entity_json: Value,
    pub revisions: Vec::<WikidataRevision>
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MongoRevision {
    pub id: u64,
    pub class_id: String,
    pub entity_id: String,
    pub parent_id: u64,
    pub timestamp: String,
    pub username: String,
    pub comment: String,
    pub entity_diff: Vec::<MongoOp>
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MongoEntity {
    pub id: u64,
    pub class_id: String,
    pub entity_id: String,
    pub entity_json: Value
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MongoOp {
    pub op: String,
    pub path: String,
    pub value: Option<Value>
}

#[derive(Debug, Deserialize)]
pub struct CSVRecord {
    pub entity_id: String,
    pub class_id: String
}