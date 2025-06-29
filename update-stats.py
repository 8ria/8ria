import os
import requests
from datetime import datetime, timedelta
import re

USERNAME = "8ria"
NOW = datetime.utcnow()
START_DATE = NOW - timedelta(days=30)
DAYS = 30

# GitHub GraphQL query with updated date range
query = """
{
  user(login: "%s") {
    contributionsCollection(from: "%s", to: "%s") {
      contributionCalendar {
        totalContributions
      }
    }
  }
}
""" % (
    USERNAME,
    START_DATE.strftime("%Y-%m-%dT00:00:00Z"),
    NOW.strftime("%Y-%m-%dT00:00:00Z"),
)

# Get GitHub token from environment
token = os.getenv("G_TOKEN")
headers = {'Authorization': f'Bearer {token}'}

# Request contributions
response = requests.post('https://api.github.com/graphql', json={'query': query}, headers=headers)
data = response.json()

# Extract contribution data
total = data["data"]["user"]["contributionsCollection"]["contributionCalendar"]["totalContributions"]
average = round(total / DAYS, 2)
timestamp = NOW.strftime("%Y-%m-%d %H:%M UTC")

# Update README.md between markers
with open("README.md", "r", encoding="utf-8") as f:
    content = f.read()

new_stats = (
    f"<!--START_STATS-->\n"
    f"- 🧮 Total contributions made during the last 30 days: **{total}**  \n"
    f"- 📊 Average contributions per day over these 30 days: **{average}**  \n"
    f"- 🕒 Last updated: **{timestamp}**\n"
    f"<!--END_STATS-->"
)

# Replace old block
updated = re.sub(r"<!--START_STATS-->.*?<!--END_STATS-->", new_stats, content, flags=re.DOTALL)

# Save the updated README
with open("README.md", "w", encoding="utf-8") as f:
    f.write(updated)

# Print for debug (shown in GitHub Actions logs)
print("\n✅ Stats block written to README:")
print(new_stats)
