use chrono::{Duration, Utc, NaiveDate};
use regex::Regex;
use reqwest::blocking::Client;
use serde_json::Value;
use std::env;
use std::fs;

#[derive(Debug)]
struct BlogPost {
    title: String,
    url: String,
}

#[derive(Debug)]
struct ContributionStats {
    total_contributions: i64,
    average_per_day: f64,
    current_streak: i64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let username = "8ria";
    let now = Utc::now();
    let start_date = now - Duration::days(30);
    let days = 30.0;

    let stats = fetch_contribution_stats(&username, start_date, now)?;

    let streak_start = now - Duration::days(365);
    let streak = calculate_contribution_streak(&username, streak_start, now)?;

    let timestamp = now.format("%Y-%m-%d").to_string();

    let latest_blog = fetch_latest_blog_post(&Client::new())?;

    let content = fs::read_to_string("README.md")?;

    let new_stats = format!(
        "<!--START_STATS-->\n\
        ### 📈 Last 30 Days Activity ({timestamp})  \n\
        - **{total}** contributions  \n\
        - **{average}** per day\n\
        ---\n\
        - **{streak}** day streak!\n\
        ---\n\
        📝 **Latest blog:** [**{blog_title}**]({blog_url})\n\
        <!--END_STATS-->",
        total = to_emoji_number(stats.total_contributions),
        average = to_emoji_number(format!("{:.2}", stats.average_per_day)),
        streak = to_emoji_number(streak),
        timestamp = timestamp,
        blog_title = latest_blog.title,
        blog_url = latest_blog.url
    );

    let re = Regex::new(r"(?s)<!--START_STATS-->.*?<!--END_STATS-->").unwrap();
    let updated = re.replace(&content, new_stats.as_str());

    fs::write("README.md", updated.as_ref())?;

    println!("\n✅ Stats block written to README:\n{}", new_stats);
    println!("\n📈 30-day stats: {} contributions, {:.2} per day", stats.total_contributions, stats.average_per_day);
    println!("🔥 Current streak: {} days", streak);
    println!("📝 Latest blog post: {} -> {}", latest_blog.title, latest_blog.url);

    Ok(())
}

fn fetch_contribution_stats(username: &str, start_date: chrono::DateTime<Utc>, end_date: chrono::DateTime<Utc>) -> Result<ContributionStats, Box<dyn std::error::Error>> {
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
        end = end_date.format("%Y-%m-%dT00:00:00Z"),
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

    let days = 30.0;
    let average = (total as f64 / days * 100.0).round() / 100.0;

    Ok(ContributionStats {
        total_contributions: total,
        average_per_day: average,
        current_streak: 0, 
    })
}

fn calculate_contribution_streak(username: &str, start_date: chrono::DateTime<Utc>, end_date: chrono::DateTime<Utc>) -> Result<i64, Box<dyn std::error::Error>> {
    println!("🔍 Calculating contribution streak...");

    let query = format!(
        r#"{{
  user(login: "{username}") {{
    contributionsCollection(from: "{start}", to: "{end}") {{
      contributionCalendar {{
        weeks {{
          contributionDays {{
            date
            contributionCount
          }}
        }}
      }}
    }}
  }}
}}"#,
        username = username,
        start = start_date.format("%Y-%m-%dT00:00:00Z"),
        end = end_date.format("%Y-%m-%dT00:00:00Z"),
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

    let weeks = json["data"]["user"]["contributionsCollection"]["contributionCalendar"]["weeks"]
        .as_array()
        .expect("Failed to parse weeks");

    let mut contribution_days = Vec::new();

    for week in weeks {
        let days = week["contributionDays"]
            .as_array()
            .expect("Failed to parse contributionDays");

        for day in days {
            let date_str = day["date"].as_str().expect("Failed to parse date");
            let count = day["contributionCount"].as_i64().expect("Failed to parse contributionCount");

            let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .expect("Failed to parse date string");

            contribution_days.push((date, count));
        }
    }

    contribution_days.sort_by_key(|&(date, _)| date);

    let today = Utc::now().date_naive();
    let mut streak = 0i64;

    let mut current_date = today;

    while let Some(pos) = contribution_days.iter().position(|(date, _)| *date == current_date) {
        let (_, count) = contribution_days[pos];

        if count > 0 {
            streak += 1;
            current_date = current_date.pred_opt().unwrap_or(current_date);
        } else {

            if streak == 0 {

                current_date = current_date.pred_opt().unwrap_or(current_date);
            } else {

                break;
            }
        }

        if current_date < start_date.date_naive() {
            break;
        }
    }

    if streak == 0 {

        for (date, count) in contribution_days.iter().rev() {
            if *count > 0 {

                let mut check_date = *date;
                let mut temp_streak = 0i64;

                for (d, c) in contribution_days.iter().rev() {
                    if *d == check_date && *c > 0 {
                        temp_streak += 1;
                        check_date = check_date.pred_opt().unwrap_or(check_date);
                    } else if *d == check_date {

                        break;
                    }
                }

                if today.signed_duration_since(*date).num_days() <= 1 {
                    streak = temp_streak;
                }
                break;
            }
        }
    }

    println!("✅ Calculated streak: {} days", streak);

    Ok(streak)
}

fn fetch_latest_blog_post(client: &Client) -> Result<BlogPost, Box<dyn std::error::Error>> {
    println!("🔍 Fetching latest blog post from 8ria.github.io...");

    let response = client
        .get("https://8ria.github.io/index.html")
        .header("User-Agent", "rust-reqwest")
        .send()?
        .error_for_status()?;

    let html_content = response.text()?;

    let post = parse_first_blog_post(&html_content)?;

    Ok(post)
}

fn parse_first_blog_post(html: &str) -> Result<BlogPost, Box<dyn std::error::Error>> {
    println!("🔍 Parsing HTML for blog posts...");

    let post_card_pattern = Regex::new(
        r#"(?s)<div class="post-card" onclick="window\.location\.href='([^']+)'">.*?<div class="post-title">([^<]+)</div>"#
    )?;

    if let Some(captures) = post_card_pattern.captures(html) {
        let url_path = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        let title = captures.get(2).map(|m| m.as_str().trim()).unwrap_or("Unknown Post");

        println!("🔗 Found URL path: '{}'", url_path);
        println!("📝 Found title: '{}'", title);

        let full_url = if url_path.starts_with("http") {
            url_path.to_string()
        } else {
            format!("https://andriak.com/{}", url_path.trim_start_matches('/'))
        };

        println!("✅ Successfully parsed: '{}' -> '{}'", title, full_url);

        return Ok(BlogPost {
            title: title.to_string(),
            url: full_url,
        });
    }

    println!("🔄 Trying fallback parsing...");

    let onclick_regex = Regex::new(r#"onclick="window\.location\.href='([^']+)'"#)?;
    let first_title_regex = Regex::new(r#"<div class="post-title">([^<]+)</div>"#)?;

    let url_path = onclick_regex.captures(html)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");

    let title = first_title_regex.captures(html)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim())
        .unwrap_or("Latest Post");

    if !url_path.is_empty() {
        let full_url = if url_path.starts_with("http") {
            url_path.to_string()
        } else {
            format!("https://andriak.com/{}", url_path.trim_start_matches('/'))
        };

        println!("📄 Fallback success: '{}' -> '{}'", title, full_url);

        return Ok(BlogPost {
            title: title.to_string(),
            url: full_url,
        });
    }

    println!("❌ All parsing attempts failed, using default");
    Ok(BlogPost {
        title: "Latest Post".to_string(),
        url: "https://andriak.com".to_string(),
    })
}

fn to_emoji_number(n: impl ToString) -> String {
    let digit_map = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '.'
    ];

    let emoji_map = [
        "0️⃣", "1️⃣", "2️⃣", "3️⃣", "4️⃣", "5️⃣", "6️⃣", "7️⃣", "🎱", "9️⃣", "•"
    ];

    n.to_string()
        .chars()
        .map(|c| {
            if let Some(pos) = digit_map.iter().position(|&d| d == c) {
                emoji_map[pos]
            } else {
                "❓"
            }
        })
        .collect::<Vec<&str>>()
        .join("")
}
