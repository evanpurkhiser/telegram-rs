# tg

A small Telegram CLI client built in Rust on top of TDLib.

[![CI](https://github.com/evanpurkhiser/telegram-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/evanpurkhiser/telegram-rs/actions/workflows/ci.yml)

>[!NOTE]
> This project is written completely using Claude. No review of the code been done.

## Features

- Authenticate with Telegram (`tg auth --phone +123...`)
- List chats, contacts, and user info
- View chat history and search messages
- Send text and media (including albums and replies)
- Output as TOON by default, or JSON with `--json`

## Quick Start

```bash
cargo run -- auth --phone +1234567890
cargo run -- chats
cargo run -- history <chat_id> --limit 50
cargo run -- send <chat_id> "hello"
```

## Build

```bash
cargo build
```
