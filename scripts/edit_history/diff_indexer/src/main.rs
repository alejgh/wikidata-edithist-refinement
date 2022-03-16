mod model;

use crate::model::{CSVRecord, MongoEntity, MongoRevision, MongoOp, WikidataItem};

use std::collections::HashMap;
use std::fs::{DirEntry, File, read_dir};
use std::io::BufReader;

use core::clone::Clone;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use mongodb::{bson::doc, options::ClientOptions, Client, Collection};
use mongodb::error::Error;
use serde_json::Value;


/// Indexes wikidata diff files into a MongoDB instance
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Input directory where the diff files are stored
    #[clap(short, long)]
    input_dir: String,
    
    /// File containing mappings from entities ids to their class id
    #[clap(short, long)]
    entities_classes_file: String,

    /// Number of documents to index in each bulk request
    #[clap(short, long, default_value_t=100)]
    bulk_size: usize
}

async fn create_client() -> Result<Client, Error> {
    let username = std::env::var("MONGO_USERNAME").unwrap_or_else(|_| "user".into());
    let password = std::env::var("MONGO_PASSWORD").unwrap_or_else(|_| "".into());
    let mongo_url = std::env::var("MONGO_URL").unwrap_or_else(|_| "localhost:27017".into());

    // Parse your connection string into an options struct
    let mut client_options =
        ClientOptions::parse(format!("mongodb://{}:{}@{}/wd_diff", username, password, mongo_url))
            .await?;

    // Manually set an option
    client_options.app_name = Some("diff_indexer".to_string());

    // Get a handle to the cluster
    return Client::with_options(client_options);
}


#[tokio::main]
async fn main() -> mongodb::error::Result<()> {
    let args = Args::parse();

    let client: Client = create_client().await.expect("Error creating MongoDB client");
    let db_name: String = std::env::var("MONGO_DB").unwrap_or_else(|_| "wd_diff".into());
 
    // Ping the server to see if you can connect to the cluster
    client
        .database(&db_name)
        .run_command(doc! {"ping": 1}, None)
        .await?;
    
    // Get a handle to a collection in the database.
    let db = client.database(&db_name);
    let entities_collection = db.collection::<MongoEntity>("wd_entities");
    let revisions_collection = db.collection::<MongoRevision>("wd_revisions");
    //let ops_collection = db.collection::<MongoOp>("wd_ops");

    let entities_classes = get_entities_classes_dict(args.entities_classes_file);

    // get files in input dir
    let file_paths = read_dir(args.input_dir).unwrap();
    let entries = file_paths
        .filter(|e| {
            return match e.as_ref().unwrap().path().extension() {
                Some(ext) => ext == "json",
                None => false
            };
        })   
        .collect::<Result<Vec<DirEntry>, _>>().expect("Error getting files from input folder");

    // set up progress bar
    let style = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {msg} {pos:>7}/{len:7} ")
        .progress_chars("##-");
    let pb = ProgressBar::new((entries.len() / args.bulk_size) as u64)
        .with_message("Number of indexes:");
    pb.set_style(style.clone());
    

    let mut entities = Vec::<WikidataItem>::with_capacity(args.bulk_size);
    let mut i = 0;
    let mut num_instances = 0;
    
    for entry in entries {
        let path = entry.path();
	    println!("{:?}", path);

        // read file contents into entities vec
        let file = File::open(&path).expect(&format!("Could not open file: {:?}", &path));
        let reader = BufReader::new(file);

        entities.append(& mut serde_json::from_reader(reader).unwrap());
        i += 1;

        if i > args.bulk_size {
            i = 0;
            insert_many(&entities_collection, &revisions_collection, &entities, &entities_classes).await;

            num_instances += entities.len();
            entities.clear();
            pb.inc(1);
        }
    }

    if entities.len() > 0 {
        num_instances += entities.len();
        insert_many(&entities_collection, &revisions_collection, &entities, &entities_classes).await;
    }

    println!("Indexed {:?} entities", num_instances);
    pb.inc(1);
    Ok(())
}

async fn insert_many(entities_collection: &Collection::<MongoEntity>,
                     revisions_collection: &Collection<MongoRevision>,
                     entities: & Vec::<WikidataItem>,
                     entities_classes: &HashMap<String, String>) {
    let mut mongo_entities = Vec::<MongoEntity>::new();
    let mut mongo_revisions = Vec::<MongoRevision>::new();
    for entity in entities {
        let class_id: String = match entities_classes.get(&entity.entity_id) {
            Some(q_id) => q_id.to_string(),
            None => {
                println!("No class for entity {}", entity.entity_id.clone());
                String::new()
            }
        };

        let m_entity = MongoEntity {id: entity.id, entity_id: entity.entity_id.clone(),
            entity_json: entity.entity_json.clone(), class_id: class_id.clone()};
        mongo_entities.push(m_entity);

        for rev in entity.revisions.clone() {
            let mut m_ops = Vec::<MongoOp>::new();
            match rev.entity_diff {
                Some(diffs) => {
                    for diff in diffs {
                        let m_op = MongoOp {
                            op: diff.op, path: diff.path, value: diff.value
                        };
                        m_ops.push(m_op);
                    }
                },
                None => ()
            }

            let m_rev = MongoRevision {id: rev.id, entity_id: entity.entity_id.clone(),
                parent_id: rev.parent_id, timestamp: rev.timestamp,
                username: rev.username, comment: rev.comment, class_id: class_id.clone(),
                entity_diff: m_ops};
            mongo_revisions.push(m_rev);
        }
    }
    
    let result = entities_collection.insert_many(mongo_entities, None).await;
    if result.is_err() {
        println!("Error inserting entities documents");
    }
    
    let result2 = revisions_collection.insert_many(mongo_revisions, None).await;
    if result2.is_err() {
        println!("Error inserting revisions documents");
    }
}

fn get_entities_classes_dict(entities_classes_file: String) -> HashMap<String, String> {
    let mut entitites_classes = HashMap::new();
    let file = File::open(&entities_classes_file).expect(&format!("Could not open file: {:?}", &entities_classes_file));
    let mut rdr = csv::Reader::from_reader(BufReader::new(file));
    for result in rdr.deserialize() {
        let record: CSVRecord = result.expect("Error parsing CSV record");
        entitites_classes.insert(
            record.entity_id,
            record.class_id
        );
    }

    return entitites_classes;
}

fn get_idx_largest_entity(vec: & Vec::<Value>) -> usize {
    let mut max_size = 0;
    let mut max_v: Option<&Value> = None;
    let mut idx = 0;
    let mut i = 0;
    for v in vec {
        if v.to_string().chars().count() * 8 > max_size {
            max_size = v.to_string().chars().count() * 8;
            max_v = Some(v);
            idx = i;
        }

        i += 1;
    }

    println!("SIZE: {}", max_size);
    match max_v {
        Some(v) => println!("El causante: {:?}", v.get("entity_id")),
        None => println!("NO")
    }

    return idx;
}
