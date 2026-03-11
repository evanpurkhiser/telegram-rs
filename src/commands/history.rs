use anyhow::Result;
use serde_json::{json, Value};
use tdlib_rs::{enums, functions};

// Helper to convert tdlib errors
fn convert_tdlib_error<T>(result: Result<T, tdlib_rs::types::Error>) -> Result<T> {
    result.map_err(|e| anyhow::anyhow!("TDLib error: {:?}", e))
}

// Convert unix timestamp to human-readable format
fn format_date(timestamp: i32) -> String {
    use std::time::{SystemTime, UNIX_EPOCH, Duration};
    
    let datetime = UNIX_EPOCH + Duration::from_secs(timestamp as u64);
    
    // Format as "Mar 10 21:30"
    if let Ok(duration) = datetime.duration_since(UNIX_EPOCH) {
        let secs = duration.as_secs();
        let datetime = chrono::DateTime::<chrono::Local>::from(
            SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
        );
        datetime.format("%b %d %H:%M").to_string()
    } else {
        timestamp.to_string()
    }
}

pub async fn run(client_id: i32, chat_id: i64, limit: i32, from_message_id: Option<i64>, json_output: bool) -> Result<()> {
    // Get the chat to find the last message ID if not provided
    let from_message_id = if let Some(msg_id) = from_message_id {
        msg_id
    } else {
        let chat_result = convert_tdlib_error(
            functions::get_chat(chat_id, client_id).await
        )?;
        
        if let enums::Chat::Chat(chat) = chat_result {
            chat.last_message.map(|msg| msg.id).unwrap_or(0)
        } else {
            0
        }
    };
    
    let history_result = convert_tdlib_error(
        functions::get_chat_history(
            chat_id,
            from_message_id, // from_message_id (start from last message)
            0, // offset
            limit,
            false, // only_local
            client_id,
        ).await
    )?;
    
    let mut messages = Vec::new();
    
    // Extract messages from the Messages enum
    if let enums::Messages::Messages(history_data) = history_result {
        for msg_option in history_data.messages.into_iter().rev() {
            if let Some(msg) = msg_option {
                let sender = match &msg.sender_id {
                    enums::MessageSender::User(user) => {
                        let user_result = convert_tdlib_error(
                            functions::get_user(user.user_id, client_id).await
                        )?;
                        
                        if let enums::User::User(user_info) = user_result {
                            format!("{} {}", user_info.first_name, user_info.last_name)
                        } else {
                            "Unknown User".to_string()
                        }
                    }
                    enums::MessageSender::Chat(chat) => {
                        let chat_result = convert_tdlib_error(
                            functions::get_chat(chat.chat_id, client_id).await
                        )?;
                        
                        if let enums::Chat::Chat(chat_info) = chat_result {
                            chat_info.title
                        } else {
                            "Unknown Chat".to_string()
                        }
                    }
                };
                
                let (content_type, text, media_info) = extract_message_info(&msg.content);
                
                // Always include the same keys for consistent TOON formatting
                let msg_obj = json!({
                    "id": msg.id,
                    "date": format_date(msg.date),
                    "sender": sender,
                    "content_type": content_type,
                    "text": text,
                    "media": media_info.unwrap_or(json!(null)),
                });
                
                messages.push(msg_obj);
            }
        }
    }
    
    let output = json!(messages);
    crate::output::print_output(&output, json_output);
    
    Ok(())
}

fn extract_message_info(content: &enums::MessageContent) -> (String, String, Option<Value>) {
    match content {
        enums::MessageContent::MessageText(text) => {
            ("text".to_string(), text.text.text.clone(), None)
        }
        enums::MessageContent::MessagePhoto(photo) => {
            let caption = photo.caption.text.clone();
            let size = photo.photo.sizes.last()
                .map(|s| s.photo.size)
                .unwrap_or(0);
            
            let media = json!({
                "type": "photo",
                "size": size,
            });
            
            ("photo".to_string(), caption, Some(media))
        }
        enums::MessageContent::MessageVideo(video) => {
            let caption = video.caption.text.clone();
            let media = json!({
                "type": "video",
                "duration": video.video.duration,
                "size": video.video.video.size,
            });
            
            ("video".to_string(), caption, Some(media))
        }
        enums::MessageContent::MessageDocument(doc) => {
            let caption = doc.caption.text.clone();
            let media = json!({
                "type": "document",
                "filename": doc.document.file_name,
                "size": doc.document.document.size,
            });
            
            ("document".to_string(), caption, Some(media))
        }
        enums::MessageContent::MessageAudio(audio) => {
            let caption = audio.caption.text.clone();
            let media = json!({
                "type": "audio",
                "duration": audio.audio.duration,
                "size": audio.audio.audio.size,
                "title": audio.audio.title,
                "performer": audio.audio.performer,
            });
            
            ("audio".to_string(), caption, Some(media))
        }
        enums::MessageContent::MessageVoiceNote(voice) => {
            let caption = voice.caption.text.clone();
            let media = json!({
                "type": "voice",
                "duration": voice.voice_note.duration,
            });
            
            ("voice".to_string(), caption, Some(media))
        }
        _ => ("other".to_string(), String::new(), None),
    }
}
