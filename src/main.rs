mod client;
mod commands;
mod config;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};

// Suppress TDLib logs using td_execute before creating the client
fn suppress_tdlib_logs() {
    // Method 1: Set log stream to empty (complete silence)
    tdlib_rs::execute(r#"{"@type":"setLogStream", "log_stream":{"@type":"logStreamEmpty"}}"#.to_string());
    
    // Method 2: Set verbosity to 0 (just to be safe)
    tdlib_rs::execute(r#"{"@type":"setLogVerbosityLevel", "new_verbosity_level":0}"#.to_string());
}

#[derive(Parser)]
#[command(name = "tg")]
#[command(about = "Telegram CLI client", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Output raw JSON instead of TOON format
    #[arg(long, global = true)]
    json: bool,
    
    /// Enable verbose TDLib debug logs
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with Telegram
    Auth {
        /// Phone number (international format, e.g., +13306220474)
        #[arg(long)]
        phone: Option<String>,
    },
    
    /// List all chats
    Chats,
    
    /// Show message history for a chat
    History {
        /// Chat ID
        chat_id: i64,
        
        /// Number of messages to fetch (default: 50)
        #[arg(short, long, default_value = "50")]
        limit: i32,
        
        /// Start from this message ID (for pagination)
        #[arg(long)]
        from_message_id: Option<i64>,
    },
    
    /// List all contacts
    ContactList,
    
    /// Send a text message to a chat
    Send {
        /// Chat ID
        chat_id: i64,
        
        /// Message text (use "-" to read from stdin)
        message: String,
    },
    
    /// Search messages in a chat
    Search {
        /// Chat ID
        chat_id: i64,
        
        /// Search query
        query: String,
        
        /// Number of results to return (default: 20)
        #[arg(short, long, default_value = "20")]
        limit: i32,
    },
    
    /// Get information about a user
    UserInfo {
        /// User ID
        user_id: i64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Suppress TDLib logs if not in verbose mode
    if !cli.verbose {
        suppress_tdlib_logs();
    }
    
    match cli.command {
        Commands::Auth { phone } => {
            let mut client = client::TelegramClient::new(cli.verbose).await?;
            client.authenticate(phone).await?;
            client.close().await?;
            Ok(())
        }
        Commands::Chats => {
            let mut client = client::TelegramClient::new(cli.verbose).await?;
            client.authenticate(None).await?;
            let result = commands::chats::run(client.client_id(), cli.json).await;
            client.close().await?;
            result
        }
        Commands::History { chat_id, limit, from_message_id } => {
            let mut client = client::TelegramClient::new(cli.verbose).await?;
            client.authenticate(None).await?;
            let result = commands::history::run(client.client_id(), chat_id, limit, from_message_id, cli.json).await;
            client.close().await?;
            result
        }
        Commands::ContactList => {
            let mut client = client::TelegramClient::new(cli.verbose).await?;
            client.authenticate(None).await?;
            let result = commands::contact_list::run(client.client_id(), cli.json).await;
            client.close().await?;
            result
        }
        Commands::Send { chat_id, message } => {
            let mut client = client::TelegramClient::new(cli.verbose).await?;
            client.authenticate(None).await?;
            
            // Read from stdin if message is "-"
            let message = if message == "-" {
                use std::io::Read;
                let mut buffer = String::new();
                std::io::stdin().read_to_string(&mut buffer)?;
                buffer
            } else {
                message
            };
            
            let result = commands::send::run(client.client_id(), chat_id, message).await;
            client.close().await?;
            result
        }
        Commands::Search { chat_id, query, limit } => {
            let mut client = client::TelegramClient::new(cli.verbose).await?;
            client.authenticate(None).await?;
            let result = commands::search::run(client.client_id(), chat_id, query, limit, cli.json).await;
            client.close().await?;
            result
        }
        Commands::UserInfo { user_id } => {
            let mut client = client::TelegramClient::new(cli.verbose).await?;
            client.authenticate(None).await?;
            let result = commands::user_info::run(client.client_id(), user_id, cli.json).await;
            client.close().await?;
            result
        }
    }
}
