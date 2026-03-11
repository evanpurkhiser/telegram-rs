mod album;
mod media;

use anyhow::Result;
use tdlib_rs::{enums, functions, types};

fn convert_tdlib_error<T>(result: Result<T, tdlib_rs::types::Error>) -> Result<T> {
    result.map_err(|e| anyhow::anyhow!("TDLib error: {:?}", e))
}

fn build_reply_to(reply_to: Option<i64>) -> Option<enums::InputMessageReplyTo> {
    reply_to.map(|message_id| {
        enums::InputMessageReplyTo::Message(types::InputMessageReplyToMessage {
            message_id,
            quote: None,
            checklist_task_id: 0,
        })
    })
}

async fn send_text_message(
    client_id: i32,
    chat_id: i64,
    text: String,
    reply_to: Option<enums::InputMessageReplyTo>,
) -> Result<Vec<i64>> {
    let content = enums::InputMessageContent::InputMessageText(types::InputMessageText {
        text: types::FormattedText {
            text,
            entities: vec![],
        },
        link_preview_options: None,
        clear_draft: false,
    });

    let sent_message = convert_tdlib_error(
        functions::send_message(chat_id, None, reply_to, None, content, client_id).await,
    )?;

    let enums::Message::Message(message) = sent_message;
    eprintln!("✓ Message sent");
    Ok(vec![message.id])
}

async fn send_single_media(
    client_id: i32,
    chat_id: i64,
    file_path: &str,
    caption: Option<types::FormattedText>,
    reply_to: Option<enums::InputMessageReplyTo>,
) -> Result<Vec<i64>> {
    let (content, media_type) = media::build_media_content(file_path, caption)?;
    let sent_message = convert_tdlib_error(
        functions::send_message(chat_id, None, reply_to, None, content, client_id).await,
    )?;

    let enums::Message::Message(message) = sent_message;
    eprintln!("Uploading {} ({})...", file_path, media_type.as_str());
    Ok(vec![message.id])
}

pub async fn run(
    client_id: i32,
    chat_id: i64,
    message: Option<String>,
    files: Vec<String>,
    reply_to: Option<i64>,
) -> Result<Vec<i64>> {
    let reply_to_message = build_reply_to(reply_to);
    let caption = message.as_ref().map(|text| types::FormattedText {
        text: text.clone(),
        entities: vec![],
    });

    if files.is_empty() {
        let text = message
            .ok_or_else(|| anyhow::anyhow!("Must provide either message text or files to send"))?;
        return send_text_message(client_id, chat_id, text, reply_to_message).await;
    }

    if files.len() == 1 {
        return send_single_media(client_id, chat_id, &files[0], caption, reply_to_message).await;
    }

    album::send_album(client_id, chat_id, files, caption, reply_to_message).await
}
