use std::fs::{DirEntry, File, read_dir};
use std::io::BufReader;

use clap::Parser;
use elasticsearch::{BulkOperation, BulkParts, DEFAULT_ADDRESS, Elasticsearch, Error};
use elasticsearch::auth::Credentials;
use elasticsearch::http::transport::{SingleNodeConnectionPool, TransportBuilder};
use elasticsearch::indices::{IndicesCreateParts, IndicesDeleteParts, IndicesExistsParts};
use http::StatusCode;
use indicatif::ProgressBar;
use serde_json::{json, Value};
use url::Url;

static ENTITIES_INDEX: &'static str = "wd_entities";


/// Indexes wikidata diff files into an ElasticSearch instance
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


#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("Connecting to elasticsearch instance");
    let client = create_client().unwrap();
    println!("Connection finished correctly");

    println!("Creating elasticsearch index");
    create_index_if_not_exists(&client, true).await.expect("Error creating index");
    println!("Index created");

    let file_paths = read_dir(args.input_dir).unwrap();
    let entries = file_paths.collect::<Result<Vec<DirEntry>, _>>().expect("Error getting files from input folder");
    
    let pb = ProgressBar::new((entries.len() / args.bulk_size + 1) as u64)
        .with_message("Number of indexes");
    

    let mut entities = Vec::<Value>::with_capacity(args.bulk_size);
    let mut i = 0;
    
    for entry in entries {
        let path = entry.path();
        match path.extension() {
            Some(ext) => {
                if ext != "json" {
                    continue;
                }
            },
            None => continue
        }

        // read file contents into entities vec
        let file = File::open(&path).expect(&format!("Could not open file: {:?}", &path));
        let reader = BufReader::new(file);

        entities.append(& mut serde_json::from_reader(reader).unwrap());
        i += 1;

        if i > args.bulk_size {
            println!("f");
            i = 0;
            index_entities(&client, &mut entities).await.expect("Error indexing entities");
            pb.inc(1);
        }
    }

    if entities.len() > 0 {
        println!("f2");
        index_entities(&client, &mut entities).await.expect("Error indexing entities");
    }

    pb.inc(1);
}

async fn index_entities(client: &Elasticsearch, entities: &mut Vec<Value>) -> Result<(), Error> {
    println!("INDEX");

    for e in entities.iter_mut() {
        let revisions: &mut Vec<Value> = e.get_mut("revisions").unwrap().as_array_mut().unwrap();
        for rev in revisions.iter_mut() {
            let diff = rev.get_mut("entity_diff").unwrap().as_array_mut().unwrap();
            for d in diff.iter_mut() {
                if d.get("value") != None && d.get("value").unwrap().as_object() == None && d.get("value").unwrap().as_array() == None {
                    let obj = d.as_object_mut().unwrap();
                    let key = "stringValue";
                    let value = obj.remove("value").unwrap();
                    obj.insert(key.to_string(), value);
                }
            }
        }
    }

    println!("2");
    let e = &entities[0];
    let id = e.get("id").unwrap().to_string();
    let b = BulkOperation::index(e).id(&id).routing(&id).into();
    let mut body: Vec<BulkOperation<_>> = Vec::new();
    body.push(b);


    //let body: Vec<BulkOperation<_>> = entities
    //    .iter()
    //    .map(|e| {
    //        let id = e.get("id").unwrap().to_string();
    //        BulkOperation::index(e).id(&id).routing(&id).into()
    //    })
    //    .collect();

    println!("3");
    let response = client
        .bulk(BulkParts::Index(ENTITIES_INDEX))
        .body(body)
        .send()
        .await?;

    println!("4");
    let json: Value = response.json().await?;

    println!("5");
    if json["errors"].as_bool().unwrap() {
        let failed: Vec<&Value> = json["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| !v["error"].is_null())
            .collect();

        println!("Errors whilst indexing. Failures: {}", failed.len());
        println!("{}", json);
    }

    Ok(())
}

async fn create_index_if_not_exists(client: &Elasticsearch, delete: bool) -> Result<(), Error> {
    let exists = client
        .indices()
        .exists(IndicesExistsParts::Index(&[ENTITIES_INDEX]))
        .send()
        .await?;

    if exists.status_code().is_success() && delete {
        let delete = client
            .indices()
            .delete(IndicesDeleteParts::Index(&[ENTITIES_INDEX]))
            .send()
            .await?;

        if !delete.status_code().is_success() {
            println!("Problem deleting index: {}", delete.text().await?);
        }
    }

    if exists.status_code() == StatusCode::NOT_FOUND || delete {
        let response = client
            .indices()
            .create(IndicesCreateParts::Index(ENTITIES_INDEX))
            .body(json!(
                {
                    "settings": {
                        "index.number_of_shards": 50,
                        "index.number_of_replicas": 0
                    }
                }
            ))
            .send()
            .await?;

        if !response.status_code().is_success() {
            println!("Error while creating index");
        }
    }

    Ok(())
}

fn create_client() -> Result<Elasticsearch, Error> {
    fn cluster_addr() -> String {
        match std::env::var("ES_URL") {
            Ok(server) => server,
            Err(_) => "http://127.0.0.1:9200".into(),
        }
    }

    println!("URL: {}", cluster_addr());

    let url = Url::parse(cluster_addr().as_ref()).unwrap();

    let username = std::env::var("ES_USERNAME").unwrap_or_else(|_| "elastic".into());
    let password = std::env::var("ES_PASSWORD").unwrap_or_else(|_| "D0r1m3PorfaPl!s".into());

    let credentials = Some(Credentials::Basic(username, password));

    let conn_pool = SingleNodeConnectionPool::new(url);
    let mut builder = TransportBuilder::new(conn_pool);

    builder = match credentials {
        Some(c) => {
            builder = builder.auth(c);

            #[cfg(any(feature = "native-tls", feature = "rustls-tls"))]
            {
                builder = builder.cert_validation(CertificateValidation::None);
            }

            builder
        }
        None => builder,
    };

    let transport = builder.build()?;
    Ok(Elasticsearch::new(transport))
}
