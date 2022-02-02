use std::fs::{DirEntry, read_dir};

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

    /// Folder where results will be saved
    #[clap(short, long)]
    output_dir: String,

    /// Number of documents to index in each bulk request
    #[clap(short, long, default_value_t=100)]
    bulk_size: usize
}


#[tokio::main]
async fn main() {
    let args = Args::parse();

    let client = create_client().unwrap();
    create_index_if_not_exists(&client, true).await.expect("Error creating index");

    let file_paths = read_dir(args.input_dir).unwrap();
    let entries = file_paths.collect::<Result<Vec<DirEntry>, _>>().expect("Error getting files from input folder");
    
    let pb = ProgressBar::new((entries.len() / args.bulk_size + 1) as u64)
        .with_message("Number of indexes");
    
    let mut entities = Vec::<Value>::with_capacity(args.bulk_size);
    
    for entry in entries {
        let path = entry.path();
        match path.extension() {
            Some(ext) => {
                if ext != "json" {
                    return;
                }
            },
            None => return
        }

        // TODO
        entities.push(serde_json::from_reader(rdr: R));

        if entities.len() > args.bulk_size {
            index_entities(&client, &entities).await.expect("Error indexing entities");
            pb.inc(1);
        }

    }

    if entities.len() > 0 {
        index_entities(&client, &entities).await.expect("Error indexing entities");
    }

    pb.inc(1);
}

async fn index_entities(client: &Elasticsearch, entities: &[Value]) -> Result<(), Error> {
    let body: Vec<BulkOperation<_>> = entities
        .iter()
        .map(|e| {
            let id = e.get("id").unwrap().to_string();
            BulkOperation::index(e).id(&id).routing(&id).into()
        })
        .collect();

    let response = client
        .bulk(BulkParts::Index(ENTITIES_INDEX))
        .body(body)
        .send()
        .await?;

    let json: Value = response.json().await?;

    if json["errors"].as_bool().unwrap() {
        let failed: Vec<&Value> = json["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| !v["error"].is_null())
            .collect();

        println!("Errors whilst indexing. Failures: {}", failed.len());
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
            Err(_) => DEFAULT_ADDRESS.into(),
        }
    }

    let mut url = Url::parse(cluster_addr().as_ref()).unwrap();

    let username = std::env::var("ES_USERNAME").unwrap_or_else(|_| "elastic".into());
    let password = std::env::var("ES_PASSWORD").unwrap_or_else(|_| "".into());

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
