use json_patch::Patch;

#[derive(Debug, Default, Clone)]
pub struct WikidataRevision {
    pub id: u64,
    pub parent_id: u64,
    pub timestamp: String,
    pub username: String,
    pub comment: String,
    pub entity_diff: Option<Patch>
}

#[derive(Debug, Default, Clone)]
pub struct WikidataItem {
    pub id: u64,
    pub entity_id: String,
    pub entity_json: Value,
    pub revisions: Vec::<WikidataRevision>
}