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

fn parse_date(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let s = s.trim();

    // 1. Try RFC 3339 (includes 'Z' or offset)
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&chrono::Utc));
    }

    // 2. Try RFC 2822 (includes offset)
    if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(s) {
        return Some(dt.with_timezone(&chrono::Utc));
    }

    // 3. Try naive format %Y-%m-%d %H:%M:%S (Treat as UTC for DB compatibility)
    if let Ok(ndt) =
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
    {
        use chrono::TimeZone;
        return Some(chrono::Utc.from_utc_datetime(&ndt));
    }

    // 4. Try naive ISO format
    if let Ok(ndt) =
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
    {
        use chrono::TimeZone;
        return Some(chrono::Utc.from_utc_datetime(&ndt));
    }

    None
}

fn parse_rss_date(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let s = s.trim();
    // For NewsPenguin RSS, naive strings are KST
    if let Ok(ndt) =
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
    {
        if let Some(kst) = chrono::FixedOffset::east_opt(9 * 3600) {
            use chrono::TimeZone;
            return kst
                .from_local_datetime(&ndt)
                .single()
                .map(|dt| dt.with_timezone(&chrono::Utc));
        }
    }
    parse_date(s)
}

async fn toot(msg: String) -> Result<()> {
    let access_token = env::var("NEWSPENGUIN_MSTD_ACCESS_TOKEN").expect(
        "You must set the NEWSPENGUIN_MSTD_ACCESS_TOKEN environment var!",
    );
    let access_token = access_token.trim();
    let access_url = env::var("NEWSPENGUIN_MSTD_API_URI")
        .unwrap_or_else(|_| "https://mstd.seungjin.net".to_string());
    let access_url = access_url.trim().trim_end_matches('/');

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
        (
            "User-Agent".to_string(),
            "newspenguin-rss-bot/0.1.0".to_string().into_bytes(),
        ),
    ];

    let url = format!("{}/api/v1/statuses", access_url);

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
    let saved_date = saved_date_str.as_ref().and_then(|s| parse_date(s));
    println!(
        "Comparing with saved date (UTC): {:?}",
        saved_date.map(|dt| dt.to_rfc3339())
    );

    let mut items = c.items;
    items.reverse(); // Process oldest items first

    for i in items {
        let pub_date = i.pub_date.as_ref().and_then(|s| parse_rss_date(s));

        if let Some(pd) = pub_date {
            if let Some(sd) = saved_date {
                if pd <= sd {
                    // Item is older or same as saved date, skip
                    continue;
                }
            }
        } else if saved_date.is_some() {
            // If we have a saved date but can't parse this item's date,
            // we skip it to be safe and avoid re-posting old content.
            println!("Skipping item with unparseable date: {:?}", i.pub_date);
            continue;
        }

        let title = i.title.clone().unwrap_or_default();
        let pub_date_display = pub_date
            .map(|dt| dt.to_rfc2822())
            .unwrap_or_else(|| i.pub_date.clone().unwrap_or_default());

        let mut description = i.description.clone().unwrap_or_default();
        if description.chars().count() > 300 {
            description =
                description.chars().take(300).collect::<String>() + "...";
        }

        let msg: String = format!(
            "{}:\n{}\n{}\n({})",
            title,
            description,
            i.link.unwrap_or_default(),
            pub_date_display
        );
        println!("Posting new article: {} ({})", title, pub_date_display);
        toot(msg).await?;
    }
    Ok(())
}

async fn magic() -> Result<()> {
    let rss_url = env::var("NEWSPENGUIN_RSS_URI").unwrap_or_else(|_| {
        "https://www.newspenguin.com/rss/allArticle.xml".to_string()
    });
    let rss_url = rss_url.trim();
    println!("Fetching RSS from: {}", rss_url);
    let a = feed(rss_url.to_string()).await?;

    let kv_key = "newspenguin-rss.last_build_date";
    let saved_date_result = db::get_kv(kv_key).await;
    let saved_date = match saved_date_result {
        Ok(val) => val,
        Err(e) => {
            eprintln!(
                "Warning: Failed to retrieve saved date from DB: {:?}",
                e
            );
            None
        }
    };
    println!("Retrieved saved date from DB: {:?}", saved_date);

    showme(a, saved_date).await?;

    // Save as "YYYY-MM-DD HH:MM:SS" in UTC
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    println!("Updating saved date in DB to current UTC: {}", now);
    db::set_kv(kv_key, &now).await?;

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
