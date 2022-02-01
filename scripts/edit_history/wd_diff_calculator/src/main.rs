mod model;
mod utils;

use crate::model::{WikidataItem, WikidataRevision};
use crate::utils::{get_entities_to_fetch, save_entities_diff};

use std::collections::HashSet;
use std::fs::{DirEntry, File, read_dir, remove_file};
use std::path::Path;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::process::{Command, Stdio};

use clap::Parser;
use indicatif::{ParallelProgressIterator, ProgressBar};
use json_patch::{diff};
use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::events::BytesStart;
use rayon::prelude::*;
use serde_json::{Value};


/// Processes Wikidata meta history dump files to calculate the diff of each entity and saves results to dir
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Input directory where the meta history dump files are stored
    #[clap(short, long)]
    input_dir: String,

    /// Folder where results will be saved
    #[clap(short, long)]
    output_dir: String,

    /// Number of entities to save in each file
    #[clap(short, long, default_value_t=100)]
    bulk_size: usize,


    /// File containing a list of entities (delimited by newline) which will be processed from the dumps
    #[clap(short, long)]
    entities_file: Option<String>,
}


fn process_file(file_name: & impl AsRef<Path>, output_dir: & impl AsRef<Path>,
                entities_to_fetch: &Option<HashSet<String>>, bulk_size: usize) {
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
    let mut item_bulk = Vec::<WikidataItem>::with_capacity(bulk_size);
    let mut bulk_counter: u8 = 0;

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
                        previous_json = None;
                        valid_entity = false;
                    },
                    b"revision" => {
                        // reset state for next revision
                        current_revision = WikidataRevision::default();
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

                        match entities_to_fetch {
                            Some(entities) => valid_entity = entities.contains(&current_item.entity_id),
                            None => valid_entity = true
                        }
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

                        if previous_json.is_none() {
                            let left = serde_json::from_str("{}").unwrap();
                            current_revision.entity_diff = Some(diff(&left, &entity_json));
                        } else {
                            current_revision.entity_diff = Some(diff(&previous_json.unwrap(), &entity_json));
                        }

                        previous_json = Some(entity_json);
                    },
                    _ => ()
                }
            },
            Ok(Event::End(ref e)) => {
                match e.name() {
                    b"page" => {
                        if !valid_entity || !valid_format {
                            continue;
                        }


                        current_item.entity_json = serde_json::to_string(&previous_json).unwrap();

                        // add entity to list and see if we can bulk index in ES
                        item_bulk.push(current_item.clone());
                        if item_bulk.len() > bulk_size {
                            save_entities_diff(&item_bulk, file_name, output_dir, &mut bulk_counter);
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

    // saving remaining entities of last bulk after EOF
    save_entities_diff(&item_bulk, file_name, output_dir, &mut bulk_counter);
    item_bulk.clear();
}


pub fn main() {
    let args = Args::parse();

    let entities_to_fetch = match args.entities_file {
        Some(f) => Some(get_entities_to_fetch(f)),
        None => None
    };

    let file_paths = read_dir(args.input_dir).unwrap();

    // Fail if any dir entry is error
    let entries = file_paths.collect::<Result<Vec<DirEntry>, _>>().expect("Error getting files from input folder");
    
    let pb = ProgressBar::new(entries.len() as u64)
        .with_message("Files processed");

    entries.par_iter().progress_with(pb).for_each(|dir_entry| {
        let path = dir_entry.path();
        match path.extension() {
            Some(ext) => {
                if ext != "7z" {
                    return;
                }
            },
            None => return
        }
        
        println!("Extracting file {:?}...", path);
        let new_filename = path.to_str().unwrap().replace(".7z", ".xml");
        let fd = File::create(&new_filename).unwrap().into_raw_fd();
        
        // from_raw_fd is only considered unsafe if the file is used for mmap
        let out = unsafe {Stdio::from_raw_fd(fd)};

        let mut child = Command::new("7z")
            .args(["e", &path.to_str().unwrap(), "-so"])
            .stdout(out)
            .spawn()
            .expect("failed to execute process");
        child.wait().unwrap();
        
        println!("File '{:?}' extracted", path);

        println!("Procesing file: {:?}", path);
        process_file(&new_filename, &args.output_dir, &entities_to_fetch, args.bulk_size);
        println!("File {:?} has been processed.", path);

        let error_msg = format!("File {} could not be deleted!", new_filename);
        remove_file(new_filename).expect(&error_msg);

    });
}
