use anyhow::Result;
use serde_json::json;
use tdlib_rs::{enums, functions};

// Helper to convert tdlib errors
fn convert_tdlib_error<T>(result: Result<T, tdlib_rs::types::Error>) -> Result<T> {
    result.map_err(|e| anyhow::anyhow!("TDLib error: {:?}", e))
}

// Convert unix timestamp to human-readable format
fn format_date(timestamp: i32) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let datetime = UNIX_EPOCH + Duration::from_secs(timestamp as u64);

    if let Ok(duration) = datetime.duration_since(UNIX_EPOCH) {
        let secs = duration.as_secs();
        let datetime = chrono::DateTime::<chrono::Local>::from(
            SystemTime::UNIX_EPOCH + Duration::from_secs(secs),
        );
        datetime.format("%b %d %H:%M").to_string()
    } else {
        timestamp.to_string()
    }
}

pub async fn run(
    client_id: i32,
    chat_id: i64,
    query: String,
    limit: i32,
    json_output: bool,
) -> Result<()> {
    let search_result = convert_tdlib_error(
        functions::search_chat_messages(
            chat_id, None, // topic_id
            query, None, // sender_id
            0,    // from_message_id
            0,    // offset
            limit, None, // filter
            client_id,
        )
        .await,
    )?;

    let mut messages = Vec::new();

    let enums::FoundChatMessages::FoundChatMessages(found_messages) = search_result;
    for msg in found_messages.messages {
        let sender = match &msg.sender_id {
            enums::MessageSender::User(user) => {
                let user_result =
                    convert_tdlib_error(functions::get_user(user.user_id, client_id).await)?;
                let enums::User::User(user_info) = user_result;
                format!("{} {}", user_info.first_name, user_info.last_name)
            }
            enums::MessageSender::Chat(chat) => {
                let chat_result =
                    convert_tdlib_error(functions::get_chat(chat.chat_id, client_id).await)?;
                let enums::Chat::Chat(chat_info) = chat_result;
                chat_info.title
            }
        };

        let text = extract_message_text(&msg.content);

        messages.push(json!({
            "id": msg.id,
            "date": format_date(msg.date),
            "sender": sender,
            "text": text,
        }));
    }

    let output = json!(messages);
    crate::output::print_output(&output, json_output);

    Ok(())
}

fn extract_message_text(content: &enums::MessageContent) -> String {
    match content {
        enums::MessageContent::MessageText(text) => text.text.text.clone(),
        enums::MessageContent::MessagePhoto(photo) => photo.caption.text.clone(),
        enums::MessageContent::MessageVideo(video) => video.caption.text.clone(),
        enums::MessageContent::MessageDocument(doc) => doc.caption.text.clone(),
        enums::MessageContent::MessageAudio(audio) => audio.caption.text.clone(),
        enums::MessageContent::MessageVoiceNote(voice) => voice.caption.text.clone(),
        _ => String::new(),
    }
}
