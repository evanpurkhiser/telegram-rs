use anyhow::Result;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tdlib_rs::{enums, functions};
use tokio::sync::{broadcast, mpsc};

use crate::config::{self, Config, Paths};

// Helper to convert tdlib errors to anyhow
fn convert_tdlib_error<T>(result: Result<T, tdlib_rs::types::Error>) -> Result<T> {
    result.map_err(|e| anyhow::anyhow!("TDLib error: {:?}", e))
}

pub struct TelegramClient {
    client_id: i32,
    paths: Paths,
    config: Config,
    run_flag: Arc<AtomicBool>,
    verbose: bool,
    update_sender: broadcast::Sender<enums::Update>,
}

impl TelegramClient {
    pub async fn new(verbose: bool) -> Result<Self> {
        let client_id = tdlib_rs::create_client();
        let paths = Paths::new()?;
        let config = config::load_config()?;

        // Create broadcast channel for updates
        let (update_sender, _) = broadcast::channel(100);

        // Start the centralized update receiver
        let run_flag = Arc::new(AtomicBool::new(true));
        let run_flag_clone = run_flag.clone();
        let sender_clone = update_sender.clone();

        tokio::spawn(async move {
            while run_flag_clone.load(Ordering::Acquire) {
                let result = tokio::task::spawn_blocking(tdlib_rs::receive)
                    .await
                    .unwrap();

                if let Some((update, _)) = result {
                    // Broadcast to all subscribers (ignore errors if no one is listening)
                    let _ = sender_clone.send(update);
                } else {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
        });

        Ok(Self {
            client_id,
            paths,
            config,
            run_flag,
            verbose,
            update_sender,
        })
    }

    pub async fn authenticate(&mut self, phone_override: Option<String>) -> Result<()> {
        let mut update_rx = self.update_sender.subscribe();
        let (auth_tx, mut auth_rx) = mpsc::channel(5);

        // Spawn task to filter auth updates from the broadcast
        tokio::spawn(async move {
            while let Ok(update) = update_rx.recv().await {
                if let enums::Update::AuthorizationState(state) = update {
                    let _ = auth_tx.send(state.authorization_state).await;
                }
            }
        });

        // Configure logging based on verbose flag
        if self.verbose {
            // Enable verbose logging to stderr
            convert_tdlib_error(
                functions::set_log_stream(enums::LogStream::Default, self.client_id).await,
            )?;
            convert_tdlib_error(functions::set_log_verbosity_level(2, self.client_id).await)?;
        } else {
            // Disable all logging
            convert_tdlib_error(
                functions::set_log_stream(enums::LogStream::Empty, self.client_id).await,
            )?;
        }

        // Handle authorization states
        while let Some(state) = auth_rx.recv().await {
            match state {
                enums::AuthorizationState::WaitTdlibParameters => {
                    self.send_tdlib_parameters().await?;
                }
                enums::AuthorizationState::WaitPhoneNumber => {
                    self.send_phone_number(phone_override.clone()).await?;
                }
                enums::AuthorizationState::WaitCode(_) => {
                    self.send_code().await?;
                }
                enums::AuthorizationState::WaitPassword(_) => {
                    self.send_password().await?;
                }
                enums::AuthorizationState::Ready => {
                    return Ok(());
                }
                enums::AuthorizationState::Closed => {
                    self.run_flag.store(false, Ordering::Release);
                    anyhow::bail!("TDLib closed unexpectedly during auth");
                }
                _ => {}
            }
        }

        anyhow::bail!("Auth channel closed unexpectedly")
    }

    async fn send_tdlib_parameters(&self) -> Result<()> {
        let api_id = self.config.api_id.unwrap_or(94575);
        let api_hash = self
            .config
            .api_hash
            .clone()
            .unwrap_or_else(|| "a3406de8d171bb422bb6ddf3bbd800e2".to_string());

        let app_version = format!("telegram-rs {}", env!("CARGO_PKG_VERSION"));

        convert_tdlib_error(
            functions::set_tdlib_parameters(
                false,                                             // use_test_dc
                self.paths.data_dir.to_string_lossy().to_string(), // database_directory
                self.paths
                    .data_dir
                    .join("files")
                    .to_string_lossy()
                    .to_string(), // files_directory
                String::new(),                                     // database_encryption_key
                true,                                              // use_file_database
                true,                                              // use_chat_info_database
                true,                                              // use_message_database
                true,                                              // use_secret_chats
                api_id,
                api_hash,
                "en".to_string(),                 // system_language_code
                "Telegram Rust CLI".to_string(),  // device_model (shows in devices list)
                std::env::consts::OS.to_string(), // system_version
                app_version,                      // application_version (telegram-rs X.Y.Z)
                self.client_id,
            )
            .await,
        )?;
        Ok(())
    }

    async fn send_phone_number(&mut self, phone_override: Option<String>) -> Result<()> {
        let phone = if let Some(p) = phone_override {
            // Use phone from command line arg
            eprintln!("Using phone from argument: {}", p);
            p
        } else if let Some(ref p) = self.config.phone {
            // Use phone from config
            eprintln!("Using phone from config: {}", p);
            p.clone()
        } else {
            // Prompt for phone
            eprint!("Enter phone number (international format, e.g., +13306220474): ");
            io::stderr().flush()?;

            let phone = tokio::task::spawn_blocking(|| {
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                input.trim().to_string()
            })
            .await?;

            if phone.is_empty() {
                anyhow::bail!("Phone number cannot be empty");
            }

            phone
        };

        // Save phone to config if not already saved
        if self.config.phone.as_ref() != Some(&phone) {
            self.config.phone = Some(phone.clone());
            config::save_config(&self.config)?;
            eprintln!("✓ Phone number saved to config");
        }

        convert_tdlib_error(
            functions::set_authentication_phone_number(
                phone,
                None, // settings
                self.client_id,
            )
            .await,
        )?;
        Ok(())
    }

    async fn send_code(&self) -> Result<()> {
        eprint!("Enter authentication code: ");
        io::stderr().flush()?;

        let code = tokio::task::spawn_blocking(|| {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            input.trim().to_string()
        })
        .await?;

        convert_tdlib_error(functions::check_authentication_code(code, self.client_id).await)?;
        Ok(())
    }

    async fn send_password(&self) -> Result<()> {
        let password =
            tokio::task::spawn_blocking(|| rpassword::prompt_password("Enter Telegram password: "))
                .await??;

        convert_tdlib_error(
            functions::check_authentication_password(password, self.client_id).await,
        )?;
        Ok(())
    }

    pub fn client_id(&self) -> i32 {
        self.client_id
    }

    /// Load chats into TDLib's cache. This should be called before any command that uses chat IDs.
    /// According to TDLib docs, chats are delivered via updateNewChat updates, but calling loadChats
    /// triggers TDLib to load them from the database.
    pub async fn load_chats(&self) -> Result<()> {
        // Load main chat list - this will trigger updateNewChat for all chats
        convert_tdlib_error(
            functions::load_chats(
                Some(enums::ChatList::Main),
                100, // limit - load first 100 chats
                self.client_id,
            )
            .await,
        )?;

        // Give TDLib time to process the updates and load chats into cache
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        Ok(())
    }

    /// Wait for messages to finish sending. This monitors updateMessageSendSucceeded.
    pub async fn wait_for_messages(&self, pending_ids: Vec<i64>) -> Result<()> {
        if pending_ids.is_empty() {
            return Ok(());
        }

        let mut update_rx = self.update_sender.subscribe();
        let (tx, mut rx) = mpsc::channel(10);

        // Spawn task to filter message send updates from the broadcast
        tokio::spawn(async move {
            while let Ok(update) = update_rx.recv().await {
                if let enums::Update::MessageSendSucceeded(update_data) = update {
                    let _ = tx.send(update_data.old_message_id).await;
                }
            }
        });

        // Wait for all messages to complete with timeout
        let timeout = std::time::Duration::from_secs(300); // 5 minute timeout
        let start = std::time::Instant::now();
        let mut remaining = pending_ids;

        while !remaining.is_empty() {
            if start.elapsed() > timeout {
                anyhow::bail!("Timeout waiting for messages to send");
            }

            tokio::select! {
                Some(completed_id) = rx.recv() => {
                    // Remove the completed message from pending list
                    if let Some(pos) = remaining.iter().position(|&id| id == completed_id) {
                        remaining.remove(pos);
                        eprintln!("✓ Upload completed ({} remaining)", remaining.len());
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                    // Continue waiting
                }
            }
        }

        eprintln!("✓ All uploads completed");
        Ok(())
    }

    pub async fn close(&mut self) -> Result<()> {
        let mut update_rx = self.update_sender.subscribe();

        // Send close request to TDLib
        convert_tdlib_error(functions::close(self.client_id).await)?;

        // Wait for TDLib to confirm it's closed (with timeout)
        let timeout = tokio::time::Duration::from_secs(5);
        let start = tokio::time::Instant::now();

        while start.elapsed() < timeout {
            tokio::select! {
                Ok(update) = update_rx.recv() => {
                    if let enums::Update::AuthorizationState(state) = update
                        && matches!(state.authorization_state, enums::AuthorizationState::Closed)
                    {
                        self.run_flag.store(false, Ordering::Release);
                        return Ok(());
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                    // Continue waiting
                }
            }
        }

        // Timeout - stop anyway
        self.run_flag.store(false, Ordering::Release);
        eprintln!("Warning: TDLib close timed out");
        Ok(())
    }
}
