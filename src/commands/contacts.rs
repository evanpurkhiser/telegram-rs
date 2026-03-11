use anyhow::Result;
use serde_json::json;
use tdlib_rs::{enums, functions};

// Helper to convert tdlib errors
fn convert_tdlib_error<T>(result: Result<T, tdlib_rs::types::Error>) -> Result<T> {
    result.map_err(|e| anyhow::anyhow!("TDLib error: {:?}", e))
}

pub async fn run(client_id: i32, json_output: bool) -> Result<()> {
    let contacts_result = convert_tdlib_error(functions::get_contacts(client_id).await)?;

    let mut contacts = Vec::new();

    let enums::Users::Users(users_data) = contacts_result;
    for user_id in users_data.user_ids {
        let user_result = convert_tdlib_error(functions::get_user(user_id, client_id).await)?;

        let enums::User::User(user) = user_result;
        contacts.push(json!({
            "id": user_id,
            "first_name": user.first_name,
            "last_name": user.last_name,
            "username": user.usernames.as_ref()
                .and_then(|u| u.active_usernames.first())
                .cloned()
                .unwrap_or_default(),
            "phone": user.phone_number,
        }));
    }

    let output = json!(contacts);
    crate::output::print_output(&output, json_output);

    Ok(())
}
