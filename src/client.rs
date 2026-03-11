use anyhow::Result;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tdlib_rs::{enums, functions};
use tokio::sync::mpsc;

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
}

impl TelegramClient {
    pub async fn new(verbose: bool) -> Result<Self> {
        let client_id = tdlib_rs::create_client();
        let paths = Paths::new()?;
        let config = config::load_config()?;
        
        Ok(Self {
            client_id,
            paths,
            config,
            run_flag: Arc::new(AtomicBool::new(true)),
            verbose,
        })
    }
    
    pub async fn authenticate(&mut self, phone_override: Option<String>) -> Result<()> {
        let (auth_tx, mut auth_rx) = mpsc::channel(5);
        
        // Spawn task to receive updates
        let run_flag = self.run_flag.clone();
        tokio::spawn(async move {
            while run_flag.load(Ordering::Acquire) {
                let result = tokio::task::spawn_blocking(tdlib_rs::receive).await.unwrap();
                
                if let Some((update, _)) = result {
                    if let enums::Update::AuthorizationState(state) = update {
                        let _ = auth_tx.send(state.authorization_state).await;
                    }
                } else {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
        });
        
        // Configure logging based on verbose flag
        if self.verbose {
            // Enable verbose logging to stderr
            convert_tdlib_error(
                functions::set_log_stream(
                    enums::LogStream::Default,
                    self.client_id,
                ).await
            )?;
            convert_tdlib_error(functions::set_log_verbosity_level(2, self.client_id).await)?;
        } else {
            // Disable all logging
            convert_tdlib_error(
                functions::set_log_stream(
                    enums::LogStream::Empty,
                    self.client_id,
                ).await
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
        let api_hash = self.config.api_hash
            .clone()
            .unwrap_or_else(|| "a3406de8d171bb422bb6ddf3bbd800e2".to_string());
        
        convert_tdlib_error(
            functions::set_tdlib_parameters(
                false, // use_test_dc
                self.paths.data_dir.to_string_lossy().to_string(), // database_directory
                self.paths.data_dir.join("files").to_string_lossy().to_string(), // files_directory
                String::new(), // database_encryption_key
                true, // use_file_database
                true, // use_chat_info_database
                true, // use_message_database
                true, // use_secret_chats
                api_id,
                api_hash,
                "en".to_string(), // system_language_code
                "telegram-rs".to_string(), // device_model (shows in devices list)
                std::env::consts::OS.to_string(), // system_version
                env!("CARGO_PKG_VERSION").to_string(), // application_version
                self.client_id,
            ).await
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
            ).await
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
        
        convert_tdlib_error(
            functions::check_authentication_code(code, self.client_id).await
        )?;
        Ok(())
    }
    
    async fn send_password(&self) -> Result<()> {
        eprint!("Enter 2FA password: ");
        io::stderr().flush()?;
        
        let password = tokio::task::spawn_blocking(|| {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            input.trim().to_string()
        })
        .await?;
        
        convert_tdlib_error(
            functions::check_authentication_password(password, self.client_id).await
        )?;
        Ok(())
    }
    
    pub fn client_id(&self) -> i32 {
        self.client_id
    }
    
    pub async fn close(&self) -> Result<()> {
        self.run_flag.store(false, Ordering::Release);
        
        // Give TDLib time to process any pending operations
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        
        // Note: We intentionally don't call functions::close() as it can hang
        // The process exit will clean up the TDLib resources
        
        Ok(())
    }
}
