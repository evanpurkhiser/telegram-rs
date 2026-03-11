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

// Extract video metadata (dimensions and duration) using ffprobe
fn get_video_metadata(path: &str) -> Result<(i32, i32, i32)> {
    match ffprobe::ffprobe(path) {
        Ok(info) => {
            // Find the first video stream
            let video_stream = info.streams.iter().find(|s| s.codec_type == Some("video".to_string()));
            
            let (width, height) = if let Some(stream) = video_stream {
                (stream.width.unwrap_or(0) as i32, stream.height.unwrap_or(0) as i32)
            } else {
                (0, 0)
            };
            
            // Get duration from format or stream
            let duration = info.format.duration
                .and_then(|d| d.parse::<f64>().ok())
                .map(|d| d as i32)
                .or_else(|| {
                    video_stream.and_then(|s| s.duration.as_ref())
                        .and_then(|d| d.parse::<f64>().ok())
                        .map(|d| d as i32)
                })
                .unwrap_or(0);
            
            Ok((width, height, duration))
        }
        Err(e) => {
            // Failed to extract metadata, log warning and use zeros
            eprintln!("Warning: Failed to extract video metadata: {}", e);
            Ok((0, 0, 0))
        }
    }
}

pub async fn run(client_id: i32, chat_id: i64, message: Option<String>, files: Vec<String>, reply_to: Option<i64>) -> Result<Vec<i64>> {
    // Build reply_to object if reply message ID is provided
    let reply_to_message = reply_to.map(|msg_id| {
        enums::InputMessageReplyTo::Message(types::InputMessageReplyToMessage {
            message_id: msg_id,
            quote: None,
            checklist_task_id: 0, // 0 means reply to the whole message
        })
    });
    
    // Track message IDs we're waiting for
    let mut pending_message_ids = Vec::new();
    
    // If files are provided, send them
    if !files.is_empty() {
        let caption = message.as_ref().map(|text| types::FormattedText {
            text: text.clone(),
            entities: vec![],
        });
        
        // Check if we should send as album (multiple files) or single message
        if files.len() > 1 {
            // Collect all input message contents for the album
            let mut input_contents = Vec::new();
            
            for (index, file_path) in files.iter().enumerate() {
                // Check file exists
                if !Path::new(file_path).exists() {
                    anyhow::bail!("File not found: {}", file_path);
                }
                
                let media_type = detect_media_type(file_path)?;
                
                let input_file = enums::InputFile::Local(types::InputFileLocal {
                    path: file_path.clone(),
                });
                
                // Only add caption to the first item in the album
                let item_caption = if index == 0 { caption.clone() } else { None };
                
                let content = match media_type {
                    "photo" => enums::InputMessageContent::InputMessagePhoto(
                        types::InputMessagePhoto {
                            photo: input_file,
                            thumbnail: None,
                            added_sticker_file_ids: vec![],
                            width: 0,
                            height: 0,
                            caption: item_caption,
                            show_caption_above_media: false,
                            self_destruct_type: None,
                            has_spoiler: false,
                        }
                    ),
                    "video" => {
                        // Extract video metadata
                        let (width, height, duration) = get_video_metadata(file_path)?;
                        
                        enums::InputMessageContent::InputMessageVideo(
                            types::InputMessageVideo {
                                video: input_file,
                                thumbnail: None,
                                added_sticker_file_ids: vec![],
                                duration,
                                width,
                                height,
                                supports_streaming: true,
                                caption: item_caption,
                                show_caption_above_media: false,
                                self_destruct_type: None,
                                has_spoiler: false,
                                cover: None,
                                start_timestamp: 0,
                            }
                        )
                    },
                    "audio" => enums::InputMessageContent::InputMessageAudio(
                        types::InputMessageAudio {
                            audio: input_file,
                            album_cover_thumbnail: None,
                            duration: 0,
                            title: String::new(),
                            performer: String::new(),
                            caption: item_caption,
                        }
                    ),
                    _ => enums::InputMessageContent::InputMessageDocument(
                        types::InputMessageDocument {
                            document: input_file,
                            thumbnail: None,
                            disable_content_type_detection: false,
                            caption: item_caption,
                        }
                    ),
                };
                
                input_contents.push(content);
                eprintln!("Queued {} ({}) for album...", file_path, media_type);
            }
            
            // Send as album
            let sent_messages = convert_tdlib_error(
                functions::send_message_album(
                    chat_id,
                    None, // topic_id
                    reply_to_message.clone(),
                    None, // options
                    input_contents,
                    client_id,
                ).await
            )?;
            
            // Extract all temporary message IDs from the Messages response
            if let enums::Messages::Messages(messages_data) = sent_messages {
                for message_option in messages_data.messages {
                    if let Some(message) = message_option {
                        pending_message_ids.push(message.id);
                    }
                }
                eprintln!("Uploading album with {} files...", files.len());
            }
        } else {
            // Single file - use sendMessage as before
            let file_path = &files[0];
            
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
                "video" => {
                    // Extract video metadata
                    let (width, height, duration) = get_video_metadata(file_path)?;
                    
                    enums::InputMessageContent::InputMessageVideo(
                        types::InputMessageVideo {
                            video: input_file,
                            thumbnail: None,
                            added_sticker_file_ids: vec![],
                            duration,
                            width,
                            height,
                            supports_streaming: true,
                            caption: caption.clone(),
                            show_caption_above_media: false,
                            self_destruct_type: None,
                            has_spoiler: false,
                            cover: None,
                            start_timestamp: 0,
                        }
                    )
                },
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
            
            let sent_msg = convert_tdlib_error(
                functions::send_message(
                    chat_id,
                    None, // topic_id
                    reply_to_message.clone(), // reply_to
                    None, // options
                    content,
                    client_id,
                ).await
            )?;
            
            // Extract the temporary message ID
            if let enums::Message::Message(msg_data) = sent_msg {
                pending_message_ids.push(msg_data.id);
                eprintln!("Uploading {} ({})...", file_path, media_type);
            }
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
        
        let sent_msg = convert_tdlib_error(
            functions::send_message(
                chat_id,
                None, // topic_id
                reply_to_message, // reply_to
                None, // options
                content,
                client_id,
            ).await
        )?;
        
        // Extract the temporary message ID
        if let enums::Message::Message(msg_data) = sent_msg {
            pending_message_ids.push(msg_data.id);
        }
        
        eprintln!("✓ Message sent");
    } else {
        anyhow::bail!("Must provide either message text or files to send");
    }
    
    Ok(pending_message_ids)
}
