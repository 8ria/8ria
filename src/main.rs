use chrono::{Duration, Utc};
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    
    let username = "8ria";
    let now = Utc::now();
    let start_date = now - Duration::days(30);
    let days = 30.0;
    
    // Fetch contribution stats
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
    
    // Fetch latest blog post
    let latest_blog = fetch_latest_blog_post(&client)?;
    
    // Read and update README.md
    let content = fs::read_to_string("README.md")?;
    
    let new_stats = format!(
        "<!--START_STATS-->\n\
        - 🧮 **{total}** contributions  \n\
        - 📊 **{average}** per day  \n\
        - 🕒 Last checked on {timestamp}\n\
        ---\n\
        📝 **Latest blog:** [**{blog_title}**]({blog_url})\n\
        <!--END_STATS-->",
        total = total,
        average = average,
        timestamp = timestamp,
        blog_title = latest_blog.title,
        blog_url = latest_blog.url
    );
    
    let re = Regex::new(r"(?s)<!--START_STATS-->.*?<!--END_STATS-->").unwrap();
    let updated = re.replace(&content, new_stats.as_str());
    
    fs::write("README.md", updated.as_ref())?;
    
    println!("\n✅ Stats block written to README:\n{}", new_stats);
    println!("\n📝 Latest blog post: {} -> {}", latest_blog.title, latest_blog.url);
    
    Ok(())
}

fn fetch_latest_blog_post(client: &Client) -> Result<BlogPost, Box<dyn std::error::Error>> {
    println!("🔍 Fetching latest blog post from 8ria.github.io...");
    
    // Fetch the index.html from GitHub Pages
    let response = client
        .get("https://8ria.github.io/index.html")
        .header("User-Agent", "rust-reqwest")
        .send()?
        .error_for_status()?;
    
    let html_content = response.text()?;
    
    // Parse the HTML to find the first blog post
    let post = parse_first_blog_post(&html_content)?;
    
    Ok(post)
}

fn parse_first_blog_post(html: &str) -> Result<BlogPost, Box<dyn std::error::Error>> {
    println!("🔍 Parsing HTML for blog posts...");
    
    // Simple approach: find the first post-card with onclick and title
    let post_card_pattern = Regex::new(
        r#"(?s)<div class="post-card" onclick="window\.location\.href='([^']+)'">.*?<div class="post-title">([^<]+)</div>"#
    )?;
    
    if let Some(captures) = post_card_pattern.captures(html) {
        let url_path = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        let title = captures.get(2).map(|m| m.as_str().trim()).unwrap_or("Unknown Post");
        
        println!("🔗 Found URL path: '{}'", url_path);
        println!("📝 Found title: '{}'", title);
        
        // Convert relative URL to full URL
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
    
    // Fallback: try to find onclick and title separately
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
