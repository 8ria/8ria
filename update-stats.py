import os
import requests
from datetime import datetime

USERNAME = "8ria"
START_DATE = datetime(2025, 5, 17)
NOW = datetime.utcnow()
DAYS = (NOW - START_DATE).days or 1

query = """
{
  user(login: "%s") {
    contributionsCollection(from: "%s") {
      contributionCalendar {
        totalContributions
      }
    }
  }
}
""" % (USERNAME, START_DATE.strftime("%Y-%m-%dT00:00:00Z"))

token = os.getenv("G_TOKEN")
headers = {'Authorization': f'Bearer {token}'}

response = requests.post('https://api.github.com/graphql', json={'query': query}, headers=headers)
data = response.json()

total = data["data"]["user"]["contributionsCollection"]["contributionCalendar"]["totalContributions"]
average = round(total / DAYS, 2)

with open("README.md", "r", encoding="utf-8") as f:
    content = f.read()

new_stats = (
    f"<!--START_STATS-->\n"
    f"🧮 Total contributions since **May 17, 2025**: **{total}**\n"
    f"📆 Days active: **{DAYS}**\n"
    f"📊 Average per day: **{average}**\n"
    f"<!--END_STATS-->"
)

import re
updated = re.sub(r"<!--START_STATS-->.*?<!--END_STATS-->", new_stats, content, flags=re.DOTALL)

with open("README.md", "w", encoding="utf-8") as f:
    f.write(updated)
