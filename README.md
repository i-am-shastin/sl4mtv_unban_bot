# SL4M_TV Unban Request Bot

A Telegram bot that manages unban requests from users and forwards them to administrators for review.

## Prerequisites

- Rust 1.70 or higher
- A Telegram Bot Token (obtained from [@BotFather](https://t.me/botfather))

## Installation

1. Clone the repository:
```bash
git clone https://github.com/i-am-shastin/sl4mtv_unban_bot.git
cd sl4mtv_unban_bot
```

2. Create a `.env` file in the project root with the following variables:

```env
TELOXIDE_TOKEN=your_bot_token_here
BOT_ADMINS=admin1_id,admin2_id # Comma-separated list of admin chat IDs
DB_FILENAME=my_database.sqlite3 # Optional, defaults to db.sqlite
```

3. Start the bot using
```bash
cargo run --release
```
or
```bash
sh restart.sh
```
