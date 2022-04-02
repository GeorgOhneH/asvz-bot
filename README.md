# asvz-bot

A telegram bot to get notified/enrolled for ASVZ lessons.

## Build the binary yourself

* install [rust](https://www.rust-lang.org/tools/install)
* create a [telegram bot](https://sendpulse.com/knowledge-base/chatbot/create-telegram-chatbot)
* set env variable: TELOXIDE_TOKEN="your api token"
* Change in bot/src/main.rs on line 30 the bot name.
* run `cargo run --release`