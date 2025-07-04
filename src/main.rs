use chrono::{Duration, Utc, DateTime, Local, NaiveDateTime};
use regex::Regex;
use reqwest::blocking::Client;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::Path;
use rand::Rng;

#[derive(Debug)]
struct BlogPost {
    title: String,
    url: String,
}

#[derive(Debug)]
struct DailySchedule {
    date: String,
    run_times: Vec<u8>, // Hours of the day (0-23)
    total_runs: u8,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    
    // Check if we should run based on random schedule
    if !should_run_now()? {
        println!("⏰ Not scheduled to run at this time. Skipping...");
        return Ok(());
    }
    
    println!("🚀 Running scheduled update...");
    
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
    let timestamp = now.format("%Y-%m-%d %H:%M UTC").to_string();
    
    let latest_blog = fetch_latest_blog_post(&client)?;
    
    let content = fs::read_to_string("README.md")?;
    
    let new_stats = format!(
        "<!--START_STATS-->\n\
        ### 📈 Last 30 Days Activity ({timestamp})  \n\
        - 🧮 **{total}** contributions  \n\
        - 📊 **{average}** per day\n\
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

fn should_run_now() -> Result<bool, Box<dyn std::error::Error>> {
    let now = Utc::now();
    let today = now.format("%Y-%m-%d").to_string();
    let current_hour = now.format("%H").to_string().parse::<u8>()?;
    
    let schedule_file = format!(".schedule_{}.json", today);
    
    // Check if we have a schedule for today
    let schedule = if Path::new(&schedule_file).exists() {
        // Load existing schedule
        let content = fs::read_to_string(&schedule_file)?;
        serde_json::from_str::<DailySchedule>(&content)?
    } else {
        // Generate new schedule for today
        let schedule = generate_daily_schedule(&today);
        
        // Save schedule to file
        let schedule_json = serde_json::to_string_pretty(&schedule)?;
        fs::write(&schedule_file, schedule_json)?;
        
        println!("📅 Generated new schedule for {}", today);
        println!("🎯 Will run {} times today at hours: {:?}", 
                 schedule.total_runs, schedule.run_times);
        
        schedule
    };
    
    // Check if current hour is in our schedule
    let should_run = schedule.run_times.contains(&current_hour);
    
    println!("⏰ Current time: {}:xx UTC", current_hour);
    println!("📋 Today's schedule: {:?}", schedule.run_times);
    println!("🤔 Should run now: {}", should_run);
    
    Ok(should_run)
}

fn generate_daily_schedule(date: &str) -> DailySchedule {
    let mut rng = rand::thread_rng();
    
    // Generate random number of runs (1-30)
    let total_runs = rng.gen_range(1..=30);
    
    // Generate random hours
    let mut run_times = Vec::new();
    for _ in 0..total_runs {
        let hour = rng.gen_range(0..24);
        if !run_times.contains(&hour) {
            run_times.push(hour);
        }
    }
    
    // Sort the hours
    run_times.sort();
    
    DailySchedule {
        date: date.to_string(),
        run_times,
        total_runs: run_times.len() as u8,
    }
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
