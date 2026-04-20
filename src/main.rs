mod db;
mod wasi_http;

use anyhow::Result;
use rss::Channel;
use std::env;
use wasi as bindings;
use wasi_http::http_request;

async fn feed(url: String) -> Result<Channel> {
    let content =
        http_request(bindings::http::types::Method::Get, &url, vec![], None)
            .await?;
    let channel = Channel::read_from(&content[..])?;
    Ok(channel)
}

async fn toot(msg: String) -> Result<()> {
    let access_token = env::var("NEWSPENGUIN_MSTD_ACCESS_TOKEN")
        .expect("You must set the NEWSPENGUIN_MSTD_ACCESS_TOKEN environment var!");
    let access_url = env::var("NEWSPENGUIN_MSTD_API_URI")
        .unwrap_or_else(|_| "https://mstd.seungjin.net".to_string());

    let body =
        format!("status={}&visibility=private", urlencoding::encode(&msg));

    let headers = vec![
        (
            "Authorization".to_string(),
            format!("Bearer {}", access_token).into_bytes(),
        ),
        (
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string().into_bytes(),
        ),
    ];

    let url = format!("{}/api/v1/statuses", access_url.trim_end_matches('/'));

    http_request(
        bindings::http::types::Method::Post,
        &url,
        headers,
        Some(body.into_bytes()),
    )
    .await?;

    println!("Message posted!");
    Ok(())
}

async fn showme(c: Channel, saved_date_str: Option<String>) -> Result<()> {
    let saved_date = saved_date_str
        .and_then(|s| chrono::DateTime::parse_from_rfc2822(&s).ok());

    for i in c.items {
        if let Some(pub_date_str) = &i.pub_date {
            if let Ok(pub_date) =
                chrono::DateTime::parse_from_rfc2822(pub_date_str)
            {
                if let Some(saved) = saved_date {
                    if pub_date <= saved {
                        continue;
                    }
                }
            }
        }

        let title = i.title.clone().unwrap_or_default();
        let msg: String = format!(
            "{}:\n{}\n{}\n({})",
            title,
            i.description.unwrap_or_default(),
            i.link.unwrap_or_default(),
            i.pub_date.unwrap_or_default()
        );
        println!("New article: {}", title);
        toot(msg).await?;
    }
    Ok(())
}

async fn magic() -> Result<()> {
    let rss_url = env::var("NEWSPENGUIN_RSS_URI")
        .unwrap_or_else(|_| "https://www.newspenguin.com/rss/allArticle.xml".to_string());
    let a = feed(rss_url).await?;

    let last_build_date = a.last_build_date().unwrap_or_default().to_string();
    let kv_key = "newspenguin-rss.last_build_date";

    let saved_date = db::get_kv(kv_key).await.ok().flatten();

    if let Some(ref saved) = saved_date {
        if saved == &last_build_date && !last_build_date.is_empty() {
            println!(
                "No new updates since last build date: {}",
                last_build_date
            );
            return Ok(());
        }
    }

    showme(a, saved_date).await?;

    if !last_build_date.is_empty() {
        db::set_kv(kv_key, &last_build_date).await?;
    }

    Ok(())
}

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    println!("Start checking");

    futures::executor::block_on(async {
        if let Err(e) = magic().await {
            eprintln!("Error: {:?}", e);
        }
    });

    println!("Done");
    Ok(())
}
