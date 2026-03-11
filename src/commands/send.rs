use anyhow::Result;
use std::path::Path;
use tdlib_rs::{enums, functions, types};

// Helper to convert tdlib errors
fn convert_tdlib_error<T>(result: Result<T, tdlib_rs::types::Error>) -> Result<T> {
    result.map_err(|e| anyhow::anyhow!("TDLib error: {:?}", e))
}

// Detect media type from file extension
fn detect_media_type(path: &str) -> Result<&'static str> {
    let path = Path::new(path);
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .ok_or_else(|| anyhow::anyhow!("No file extension found for {}", path.display()))?;
    
    match ext.as_str() {
        // Images
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "heic" => Ok("photo"),
        
        // Videos
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "flv" | "m4v" => Ok("video"),
        
        // Audio
        "mp3" | "wav" | "ogg" | "m4a" | "flac" | "aac" | "opus" => Ok("audio"),
        
        // Default to document for everything else
        _ => Ok("document"),
    }
}

pub async fn run(client_id: i32, chat_id: i64, message: Option<String>, files: Vec<String>) -> Result<()> {
    // If files are provided, send them
    if !files.is_empty() {
        let caption = message.as_ref().map(|text| types::FormattedText {
            text: text.clone(),
            entities: vec![],
        });
        
        for file_path in &files {
            // Check file exists
            if !Path::new(file_path).exists() {
                anyhow::bail!("File not found: {}", file_path);
            }
            
            let media_type = detect_media_type(file_path)?;
            
            let input_file = enums::InputFile::Local(types::InputFileLocal {
                path: file_path.clone(),
            });
            
            let content = match media_type {
                "photo" => enums::InputMessageContent::InputMessagePhoto(
                    types::InputMessagePhoto {
                        photo: input_file,
                        thumbnail: None,
                        added_sticker_file_ids: vec![],
                        width: 0,
                        height: 0,
                        caption: caption.clone(),
                        show_caption_above_media: false,
                        self_destruct_type: None,
                        has_spoiler: false,
                    }
                ),
                "video" => enums::InputMessageContent::InputMessageVideo(
                    types::InputMessageVideo {
                        video: input_file,
                        thumbnail: None,
                        added_sticker_file_ids: vec![],
                        duration: 0,
                        width: 0,
                        height: 0,
                        supports_streaming: true,
                        caption: caption.clone(),
                        show_caption_above_media: false,
                        self_destruct_type: None,
                        has_spoiler: false,
                        cover: None,
                        start_timestamp: 0,
                    }
                ),
                "audio" => enums::InputMessageContent::InputMessageAudio(
                    types::InputMessageAudio {
                        audio: input_file,
                        album_cover_thumbnail: None,
                        duration: 0,
                        title: String::new(),
                        performer: String::new(),
                        caption: caption.clone(),
                    }
                ),
                _ => enums::InputMessageContent::InputMessageDocument(
                    types::InputMessageDocument {
                        document: input_file,
                        thumbnail: None,
                        disable_content_type_detection: false,
                        caption: caption.clone(),
                    }
                ),
            };
            
            convert_tdlib_error(
                functions::send_message(
                    chat_id,
                    None, // topic_id
                    None, // reply_to
                    None, // options
                    content,
                    client_id,
                ).await
            )?;
            
            eprintln!("✓ Sent {} ({})", file_path, media_type);
        }
    } else if let Some(text) = message {
        // Send text message only
        let text_content = types::FormattedText {
            text,
            entities: vec![],
        };
        
        let content = enums::InputMessageContent::InputMessageText(
            types::InputMessageText {
                text: text_content,
                link_preview_options: None,
                clear_draft: false,
            }
        );
        
        convert_tdlib_error(
            functions::send_message(
                chat_id,
                None, // topic_id
                None, // reply_to
                None, // options
                content,
                client_id,
            ).await
        )?;
        
        eprintln!("✓ Message sent");
    } else {
        anyhow::bail!("Must provide either message text or files to send");
    }
    
    Ok(())
}
