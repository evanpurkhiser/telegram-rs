use anyhow::Result;
use serde_json::json;
use tdlib_rs::{enums, functions};

// Helper to convert tdlib errors
fn convert_tdlib_error<T>(result: Result<T, tdlib_rs::types::Error>) -> Result<T> {
    result.map_err(|e| anyhow::anyhow!("TDLib error: {:?}", e))
}

pub async fn run(client_id: i32, json_output: bool) -> Result<()> {
    let chats_result = convert_tdlib_error(functions::get_chats(None, 100, client_id).await)?;

    let mut dialogs = Vec::new();

    let enums::Chats::Chats(chats_data) = chats_result;
    for chat_id in chats_data.chat_ids {
        let chat_result = convert_tdlib_error(functions::get_chat(chat_id, client_id).await)?;

        let enums::Chat::Chat(chat) = chat_result;
        let last_message_text = if let Some(ref last_msg) = chat.last_message {
            extract_message_text(&last_msg.content)
        } else {
            String::new()
        };

        dialogs.push(json!({
            "id": chat_id,
            "title": chat.title,
            "unread": chat.unread_count,
            "last_message": last_message_text,
        }));
    }

    let output = json!(dialogs);
    crate::output::print_output(&output, json_output);

    Ok(())
}

fn extract_message_text(content: &enums::MessageContent) -> String {
    match content {
        enums::MessageContent::MessageText(text) => text.text.text.clone(),
        enums::MessageContent::MessagePhoto(_) => "[Photo]".to_string(),
        enums::MessageContent::MessageVideo(_) => "[Video]".to_string(),
        enums::MessageContent::MessageDocument(_) => "[Document]".to_string(),
        enums::MessageContent::MessageAudio(_) => "[Audio]".to_string(),
        enums::MessageContent::MessageVoiceNote(_) => "[Voice]".to_string(),
        enums::MessageContent::MessageSticker(_) => "[Sticker]".to_string(),
        enums::MessageContent::MessageAnimation(_) => "[Animation]".to_string(),
        _ => "[Message]".to_string(),
    }
}
