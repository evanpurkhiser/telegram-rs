use anyhow::Result;
use tdlib_rs::{enums, functions, types};

use super::media;

pub async fn send_album(
    client_id: i32,
    chat_id: i64,
    files: Vec<String>,
    caption: Option<types::FormattedText>,
    reply_to: Option<enums::InputMessageReplyTo>,
) -> Result<Vec<i64>> {
    let mut input_contents = Vec::with_capacity(files.len());

    for (index, path) in files.iter().enumerate() {
        let item_caption = if index == 0 { caption.clone() } else { None };
        let (content, media_type) = media::build_media_content(path, item_caption)?;
        input_contents.push(content);
        eprintln!("Queued {} ({}) for album...", path, media_type.as_str());
    }

    let sent_messages = super::convert_tdlib_error(
        functions::send_message_album(chat_id, None, reply_to, None, input_contents, client_id)
            .await,
    )?;

    let enums::Messages::Messages(messages) = sent_messages;
    let pending_message_ids = messages
        .messages
        .into_iter()
        .flatten()
        .map(|message| message.id)
        .collect::<Vec<_>>();

    eprintln!("Uploading album with {} files...", files.len());
    Ok(pending_message_ids)
}
