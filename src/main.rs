// Copyright 2026 Seungjin Kim
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

mod db;
mod wasi_http;

use anyhow::Result;
use rss::Channel;
use std::env;
use wasi as bindings;
use wasi_http::http_request;

async fn feed(url: String) -> Result<Channel> {
    let user_agent = env::var("NEWSPENGUIN_USER_AGENT").unwrap_or_else(|_| {
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:125.0) Gecko/20100101 Firefox/125.0".to_string()
    });

    let headers = vec![
        (
            "User-Agent".to_string(),
            user_agent.into_bytes(),
        ),
        (
            "Accept".to_string(),
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8".to_string().into_bytes(),
        ),
        (
            "Accept-Language".to_string(),
            "en-US,en;q=0.9".to_string().into_bytes(),
        ),
        (
            "Cache-Control".to_string(),
            "max-age=0".to_string().into_bytes(),
        ),
        (
            "Upgrade-Insecure-Requests".to_string(),
            "1".to_string().into_bytes(),
        ),
    ];
    let content =
        http_request(bindings::http::types::Method::Get, &url, headers, None)
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

async fn toot(msg: String, dry_run: bool) -> Result<()> {
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string());
    if environment == "development" {
        println!("Development mode: Skipping Mastodon post.");
        println!("Message would have been:\n{}", msg);
        return Ok(());
    }

    if dry_run {
        println!("Dry run: Would post message:\n{}", msg);
        return Ok(());
    }

    let access_token = env::var("NEWSPENGUIN_MSTD_ACCESS_TOKEN").expect(
        "You must set the NEWSPENGUIN_MSTD_ACCESS_TOKEN environment var!",
    );
    let access_token = access_token.trim();
    let access_url = env::var("NEWSPENGUIN_MSTD_API_URI")
        .unwrap_or_else(|_| "https://mstd.seungjin.net".to_string());
    let access_url = access_url.trim().trim_end_matches('/');

    let body =
        format!("status={}&visibility=public", urlencoding::encode(&msg));

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

async fn showme(
    app_id: i64,
    c: Channel,
    saved_date_str: Option<String>,
    dry_run: bool,
) -> Result<()> {
    let now = chrono::Utc::now();
    let two_hour_ago = now - chrono::Duration::hours(2);

    let saved_date = saved_date_str.as_ref().and_then(|s| parse_date(s));
    println!(
        "Comparing with saved date (UTC): {:?}, and 2-hour limit: {}",
        saved_date.map(|dt| dt.to_rfc3339()),
        two_hour_ago.to_rfc3339()
    );

    let mut items = c.items;
    items.reverse(); // Process oldest items first

    for i in items {
        let link = i.link.clone().unwrap_or_default();
        if link.is_empty() {
            continue;
        }

        let pub_date = i.pub_date.as_ref().and_then(|s| parse_rss_date(s));

        // 1. 2-hour limit check
        if let Some(pd) = pub_date {
            if pd < two_hour_ago {
                println!(
                    "Skipping article older than 2 hours: {} ({})",
                    i.title.as_ref().unwrap_or(&"".to_string()),
                    pd.to_rfc3339()
                );
                continue;
            }

            if let Some(sd) = saved_date {
                if pd <= sd {
                    // Item is older or same as saved date, skip
                    continue;
                }
            }
        } else {
            // If we can't parse the date, we skip it to satisfy "DO NOT POST... more than 2 hours ago"
            println!("Skipping item with unparseable date: {:?}", i.pub_date);
            continue;
        }

        // 2. Duplicate check (Key is "posted link", Value is "link:<url>")
        match db::check_link_published(app_id, &link).await {
            Ok(true) => {
                // Link already posted, skip
                continue;
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to check KV for link {}: {:?}",
                    link, e
                );
            }
            _ => {}
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
            title, description, link, pub_date_display
        );
        println!(
            "Posting new article: {} ({})",
            title, pub_date_display
        );
        toot(msg, dry_run).await?;

        // 3. Save to KV store (key: "posted link", value: "link:<url>")
        if !dry_run {
            if let Err(e) = db::add_posted_link(app_id, &link).await {
                eprintln!("Warning: Failed to save posted link to DB: {:?}", e);
            }
        }
    }
    Ok(())
}

async fn magic(dry_run: bool) -> Result<()> {
    let app_id = env::var("APP_ID")
        .unwrap_or_else(|_| "0".to_string())
        .parse::<i64>()
        .unwrap_or(0);
    println!("Running for app_id: {}", app_id);

    let rss_url = env::var("NEWSPENGUIN_RSS_URI").unwrap_or_else(|_| {
        "https://www.newspenguin.com/rss/allArticle.xml".to_string()
    });
    let rss_url = rss_url.trim();
    println!("Fetching RSS from: {}", rss_url);
    let a = feed(rss_url.to_string()).await?;

    let kv_key = "newspenguin-rss.last_build_date";
    let saved_date_result = db::get_kv(app_id, kv_key).await;
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

    showme(app_id, a, saved_date, dry_run).await?;

    if !dry_run {
        // Save as "YYYY-MM-DD HH:MM:SS" in UTC
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        println!("Updating saved date in DB to current UTC: {}", now);
        db::set_kv(app_id, kv_key, &now).await?;

        println!("Cleaning up posted links older than a week...");
        if let Err(e) = db::delete_old_posted_messages(app_id).await {
            eprintln!("Warning: Failed to clean up old posted links: {:?}", e);
        }
    } else {
        println!("Dry run: Skipping DB updates and cleanup.");
    }

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let dry_run = args.iter().any(|arg| arg == "--dryrun");

    if dry_run {
        println!("Running in DRY RUN mode");
    }

    println!("Start checking");

    futures::executor::block_on(async {
        if let Err(e) = magic(dry_run).await {
            eprintln!("Error: {:?}", e);
        }
    });

    println!("Done");
    Ok(())
}
