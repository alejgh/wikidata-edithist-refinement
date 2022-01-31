mod model;

use crate::model::{WikidataItem, WikidataRevision};

use std::fs::File;
use std::path::Path;
use std::time::Instant;

use json_patch::{diff};
use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::events::BytesStart;
use serde_json::{Value};

fn write_patch(timestamp: &str, title: &str, patch: &json_patch::Patch) {
    let filename = format!("{}.json", timestamp);
    let folder = format!("./{}", title);

    let path = Path::new(&folder).join(filename);
    let display = path.display();
    
    let file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    serde_json::to_writer(file, patch);
}


fn process_file(file_name: impl AsRef<Path>) {
    let elastic_bulk_length = 100;

    let mut xml_reader = Reader::from_file(file_name).unwrap();
    xml_reader.trim_text(true);

    let mut buf = Vec::new();

    // keep tag state to then fetch text correctly
    let mut current_tag: &[u8] = b"";
    let mut owned_name;

    // know if we should process the given entity and if the format valid (application/json)
    let mut valid_entity: bool = false;
    let mut valid_format: bool = false;

    // keep state of previous entity json to compute diff
    let mut previous_json: Option<Value> = None;

    // keep state of current Wikidata item and revision being parsed
    let mut current_item = WikidataItem::default();
    let mut current_revision = WikidataRevision::default();

    // bulk of items to index in ElasticSearch
    let mut item_bulk = Vec::<WikidataItem>::new();

    // The `Reader` does not implement `Iterator` because it outputs borrowed data (`Cow`s)
    loop {
        match xml_reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                owned_name = BytesStart::owned_name(e.name());
                current_tag = owned_name.name().clone();

                match current_tag {
                    b"page" => {
                        // reset state for next page
                        current_item = WikidataItem::default();
                        valid_entity = false;
                    },
                    b"revision" => {
                        // reset state for next revision
                        current_revision = WikidataRevision::default();
                        previous_json = None;
                        valid_format = false;
                    },
                    _ => ()
                }
            },
            Ok(Event::Text(e)) => {
                match current_tag {
                    b"title" => {
                        current_item.entity_id = e.unescape_and_decode(&xml_reader).unwrap();
                        println!("title: {:?}", current_item.entity_id);

                        valid_entity = entities_to_fetch.contains(current_item.entity_id);
                    },
                    b"format" => {
                        let content_type = e.unescape_and_decode(&xml_reader).unwrap();
                        valid_format = content_type == "application/json";
                    },
                    b"timestamp" => {
                        current_revision.timestamp = e.unescape_and_decode(&xml_reader).unwrap();
                    },
                    b"text" => {
                        if !valid_entity || !valid_format {
                            continue;
                        }
                        
                        let entity_json: Value = serde_json::from_str(&e.unescape_and_decode(&xml_reader).unwrap()).unwrap();

                        match previous_json {
                            Some(js) => current_revision.entity_diff = Some(diff(&js, &entity_json)),
                            None => {
                                let left = serde_json::from_str("{}").unwrap();
                                current_revision.entity_diff = Some(diff(&left, &entity_json));
                            }
                        }

                        //write_patch(&current_timestamp, &current_title, &patch);
                        previous_json = Some(entity_json);
                    },
                    _ => ()
                }
            },
            Ok(Event::End(ref e)) => {
                match e.name() {
                    b"page" => {
                        // add entity to list and see if we can bulk index in ES
                        item_bulk.push(current_item.clone());

                        if item_bulk.len() > elastic_bulk_length {
                            // TODO: index to ES
                            item_bulk.clear();
                        }
                    },
                    b"revision" => {
                        current_item.revisions.push(current_revision.clone());
                    },
                    _ => ()
                }
            }
            Ok(Event::Eof) => break, // exits the loop when reaching end of file
            Err(e) => panic!("Error at position {}: {:?}", xml_reader.buffer_position(), e),
            _ => ()
        }

        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }

    // TODO: index to ES remaining items after finishing the file (last bulk)
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = Path::new("./wikidata_a.xml");
    let entities_to_fetch = get_entities_to_fetch();

    let transport = Transport::single_node("https://example.com")?;
    let client = Elasticsearch::new(transport); 

    let now = Instant::now();
    process_file(file);
    let elapsed_time = now.elapsed();

    println!("Time taken to execute program: {} seconds.", elapsed_time.as_secs());
}
