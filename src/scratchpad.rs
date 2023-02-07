use bson::Document;
use chrono::{TimeZone, Utc};
use mongodb::bson::doc;
use mongodb::{
    options::{ClientOptions, ResolverConfig},
    Client, Collection,
};
use std::env;
use std::error::Error;
use tokio;

use log::{debug, error, info, trace, warn};

async fn connect() -> Result<Client, Box<dyn Error>> {
    let client_uri =
        env::var("MONGODB_URI").expect("You must set the MONGODB_URI environment var!");
    let options =
        ClientOptions::parse_with_resolver_config(&client_uri, ResolverConfig::cloudflare())
            .await?;
    let client = Client::with_options(options)?;
    return Ok(client);
}

pub async fn write_title(title: String) -> Result<(), Box<dyn Error>> {
    let client: Client = connect().await?;
    let new_doc = doc! {
        "_app_": env::current_exe().unwrap().file_name().unwrap().to_str() ,
        "title": title,
        "created_at": chrono::Utc::now(),
    };
    let item = client.database("scratchpad").collection("scratchpad");
    let insert_result = item.insert_one(new_doc.clone(), None).await?;
    Ok(())
}

pub async fn new_title(title: String) -> Result<bool, Box<dyn Error>> {
    let client: Client = connect().await?;
    let item: Collection<Document> = client.database("scratchpad").collection("scratchpad");
    let a = item
        .count_documents(
            doc! {
                "_app_": env::current_exe().unwrap().file_name().unwrap().to_str() ,
                "title" : title,
            },
            None,
        )
        .await?;
    Ok(a != 0)
}

async fn mongotest() -> Result<(), Box<dyn Error>> {
    let client: Client = connect().await?;

    debug!("Databases:");
    for name in client.list_database_names(None, None).await? {
        println!("- {:#?}", name);
    }

    let new_doc = doc! {
        "title": "arsars",
        "created_at": chrono::Utc::now(),
    };
    debug!("{}", new_doc);
    let item = client.database("feeds").collection("newspenguin");
    let insert_result = item.insert_one(new_doc.clone(), None).await?;
    debug!("New document ID: {}", insert_result.inserted_id);

    let a = item
        .find_one(doc! { "title" : "foo"}, None)
        .await?
        .expect("arsarsars");

    debug!("{}", a);

    Ok(())
}

#[tokio::test]
async fn test_mongodbtest() {
    write_title("hello".to_string()).await;
    println!("{:#?}", check_title("hello".to_string()).await.unwrap());
}
