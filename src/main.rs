use chrono::{Duration, Utc};
use regex::Regex;
use reqwest::blocking::Client;
use serde_json::Value;
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let username = "8ria";
    let now = Utc::now();
    let start_date = now - Duration::days(30);
    let days = 30.0;

    let query = format!(
        r#"{{
  user(login: "{username}") {{
    contributionsCollection(from: "{start}", to: "{end}") {{
      contributionCalendar {{
        totalContributions
      }}
    }}
  }}
}}"#,
        username = username,
        start = start_date.format("%Y-%m-%dT00:00:00Z"),
        end = now.format("%Y-%m-%dT00:00:00Z"),
    );

    let token = env::var("G_TOKEN")
        .expect("G_TOKEN environment variable not set");

    let client = Client::new();

    let res = client
        .post("https://api.github.com/graphql")
        .bearer_auth(token)
        .header("User-Agent", "rust-reqwest")
        .json(&serde_json::json!({ "query": query }))
        .send()?
        .error_for_status()?;

    let json: Value = res.json()?;

    let total = json["data"]["user"]["contributionsCollection"]["contributionCalendar"]["totalContributions"]
        .as_i64()
        .expect("Failed to parse totalContributions");

    let average = (total as f64 / days * 100.0).round() / 100.0;

    let timestamp = now.format("%Y-%m-%d").to_string();

    let content = fs::read_to_string("README.md")?;

    let new_stats = format!(
        "<!--START_STATS-->\n\
        - 🧮 **{total}** contributions  \n\
        - 📊 **{average}** per day  \n\
        - 🕒 Last checked on {timestamp}\n\
        <!--END_STATS-->",
        total = total,
        average = average,
        timestamp = timestamp
    );

    let re = Regex::new(r"(?s)<!--START_STATS-->.*?<!--END_STATS-->").unwrap();

    let updated = re.replace(&content, new_stats.as_str());

    fs::write("README.md", updated.as_ref())?;

    println!("\n✅ Stats block written to README:\n{}", new_stats);

    Ok(())
}
