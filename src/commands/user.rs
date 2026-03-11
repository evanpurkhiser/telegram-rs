use anyhow::Result;
use serde_json::json;
use tdlib_rs::{enums, functions};

// Helper to convert tdlib errors
fn convert_tdlib_error<T>(result: Result<T, tdlib_rs::types::Error>) -> Result<T> {
    result.map_err(|e| anyhow::anyhow!("TDLib error: {:?}", e))
}

pub async fn run(client_id: i32, user_id: i64, json_output: bool) -> Result<()> {
    let user_result = convert_tdlib_error(
        functions::get_user(user_id, client_id).await
    )?;
    
    if let enums::User::User(user) = user_result {
        let username = user.usernames.as_ref()
            .and_then(|u| u.active_usernames.first())
            .cloned()
            .unwrap_or_default();
        
        let status = match &user.status {
            enums::UserStatus::Online(_) => "online",
            enums::UserStatus::Offline(_) => "offline",
            enums::UserStatus::Recently(_) => "recently",
            enums::UserStatus::LastWeek(_) => "last week",
            enums::UserStatus::LastMonth(_) => "last month",
            enums::UserStatus::Empty => "unknown",
        };
        
        let user_info = json!({
            "id": user_id,
            "first_name": user.first_name,
            "last_name": user.last_name,
            "username": username,
            "phone": user.phone_number,
            "status": status,
            "is_contact": user.is_contact,
            "is_mutual_contact": user.is_mutual_contact,
            "is_premium": user.is_premium,
        });
        
        crate::output::print_output(&user_info, json_output);
    }
    
    Ok(())
}
