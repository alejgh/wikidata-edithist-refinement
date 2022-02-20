use std::fs::{DirEntry, File, read_dir};
use std::io::BufReader;

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
    let collection = db.collection::<Value>("wd_entities");

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
    

    let mut entities = Vec::<Value>::with_capacity(args.bulk_size);
    let mut i = 0;
    
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
            let mut success = false;
            while !success {
                let res = insert_many(&collection, &entities).await;
                if res.is_some() {
                    entities.remove(res.unwrap());
                } else {
                    success = true;
                }
            }

            entities.clear();
            pb.inc(1);
        }
    }

    if entities.len() > 0 {
        let mut success = false;
        while !success {
            let res = insert_many(&collection, &entities).await;
            if res.is_some() {
                entities.remove(res.unwrap());
            } else {
                success = true;
            }
        }
    }

    pb.inc(1);
    Ok(())
}

async fn insert_many(collection: &Collection::<Value>, entities: & Vec::<Value>) -> Option<usize> {
    let result = collection.insert_many(entities, None).await;
    if result.is_err() {
        println!("Error inserting documents");
        return Some(get_idx_largest_entity(&entities));
    } else {
        return None;
    }
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
