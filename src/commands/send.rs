use anyhow::Result;
use tdlib_rs::{enums, functions, types};

// Helper to convert tdlib errors
fn convert_tdlib_error<T>(result: Result<T, tdlib_rs::types::Error>) -> Result<T> {
    result.map_err(|e| anyhow::anyhow!("TDLib error: {:?}", e))
}

pub async fn run(client_id: i32, chat_id: i64, message: String) -> Result<()> {
    // Create text message content
    let text = types::FormattedText {
        text: message,
        entities: vec![],
    };
    
    let content = enums::InputMessageContent::InputMessageText(
        types::InputMessageText {
            text,
            link_preview_options: None,
            clear_draft: false,
        }
    );
    
    // Send the message
    convert_tdlib_error(
        functions::send_message(
            chat_id,
            0, // message_thread_id
            None, // reply_to
            None, // options
            None, // reply_markup
            content,
            client_id,
        ).await
    )?;
    
    eprintln!("✓ Message sent");
    
    Ok(())
}
