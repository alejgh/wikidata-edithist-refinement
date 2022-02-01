use crate::model::{WikidataItem};

use std::fs::File;
use std::io::{BufReader, BufRead};
use std::collections::HashSet;
use std::path::Path;


pub fn get_entities_to_fetch(file_name: impl AsRef<Path>) -> HashSet<String> {
    let mut entities_to_fetch = HashSet::new();

    let file = File::open(file_name).expect("entities file not found");
    let reader = BufReader::new(file);

    for line in reader.lines() {
        entities_to_fetch.insert(String::from(line.unwrap().trim()));
    }

    return entities_to_fetch;
}

pub fn save_entities_diff(item_bulk: &Vec::<WikidataItem>, file_name: impl AsRef<Path>,
                          output_dir: impl AsRef<Path>,current_counter: &mut u8) {
    let final_filename = format!("{}_{}.json", file_name.as_ref().file_name().unwrap().to_str().unwrap(),
                                 current_counter);

    let path = Path::new(&output_dir.as_ref().as_os_str()).join(final_filename.replace("xml", "json"));
    let display = path.display();
    
    let file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    serde_json::to_writer(file, item_bulk);

    *current_counter += 1;
}
