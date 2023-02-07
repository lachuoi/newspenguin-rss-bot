mod scratchpad;

use chrono;
use rss::Channel;
use std::env;
use std::error::Error;
use std::io;
use std::io::prelude::*;
use std::string::ToString;
use tokio;

use reqwest::header::AUTHORIZATION;

use log::{debug, error, info, trace, warn};
use log4rs;
use serde_yaml;

const HOWOFTEN: i64 = 10;
const RSS_ADD: &str = "https://www.newspenguin.com/rss/allArticle.xml";

async fn feed(url: String) -> Result<Channel, Box<dyn Error>> {
    let content = reqwest::get(url).await?.bytes().await?;
    let channel = Channel::read_from(&content[..])?;
    Ok(channel)
}

async fn toot(msg: String) {
    let ACCESS_TOKEN = env::var("MSTDN_ACCESS_TOKEN")
        .expect("You must set the MSTDN_ACCESS_TOKEN environment var!");

    let res = reqwest::Client::new()
        .post("https://mstd.seungjin.net/api/v1/statuses")
        .header(AUTHORIZATION, format!("Bearer {}", ACCESS_TOKEN))
        .form(&[("status", msg), ("visibility", "private".to_string())])
        .send()
        .await;
    match res {
        Ok(_) => info!("Message posted!"),
        Err(e) => error!("Error on posting message"),
    }
}

async fn showme(c: Channel) {
    for i in c.items {
        if scratchpad::new_title(i.clone().title.unwrap())
            .await
            .unwrap()
        {
            continue;
        }

        scratchpad::write_title(i.clone().title.unwrap()).await;
        let msg: String = format!(
            "{}:\n{}\n{}\n({})",
            i.title.unwrap(),
            i.description.unwrap(),
            i.link.unwrap(),
            i.pub_date.unwrap()
        );
        info!("New article: {}", msg);
        toot(msg).await;
    }
}

async fn magic() {
    let a = feed(RSS_ADD.to_string()).await.unwrap();
    showme(a).await;
}

#[tokio::main]
async fn main() {
    let config_str = include_str!("log4rs.yaml");
    let config = serde_yaml::from_str(config_str).unwrap();
    log4rs::init_raw_config(config).unwrap();

    let mut interval_timer =
        tokio::time::interval(chrono::Duration::minutes(HOWOFTEN).to_std().unwrap());
    loop {
        // Wait for the next interval tick
        info!("Start checking");
        interval_timer.tick().await;
        tokio::spawn(async {
            magic().await;
        }); // For async task
            //tokio::task::spawn_blocking(|| do_my_task()); // For blocking task
    }
}
