use std::collections::HashSet;

pub fn get_entities_to_fetch() {
    let mut entities_to_fetch = HashSet::new();

    let file = File::open(file_name).expect("file not found!");
    let reader = BufReader::new(file);

    for line in reader.lines() {
        entities_to_fetch.insert(String::from(line.trim()));
    }

    return entities_to_fetch;
}