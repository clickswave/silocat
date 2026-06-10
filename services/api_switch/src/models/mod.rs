use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::JsonValue;
use sqlx::FromRow;
use std::option::Option;

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub api_key: String,
    pub created_on: DateTime<Utc>,
    pub account_type: String,

    pub profile_image: Option<String>,
    pub email_verified: bool,
    pub otp: String,
    pub otp_last_sent_at: Option<DateTime<Utc>>,

    pub sessions: Vec<String>,
    pub is_restricted: bool,

    pub team_id: Option<String>,
    pub subscription_id: Option<String>,

    pub default_storage_bytes: i64,

    pub transactions: Vec<JsonValue>,
    pub country: Option<String>,
    pub bio: Option<String>,

    pub username_change_count: i32,
    pub username_change_window_start: Option<DateTime<Utc>>,

    pub is_banned: bool,
    pub banned_until: Option<DateTime<Utc>>,
    pub ban_reason: Option<String>,

    pub pending_email: Option<String>,
    pub pending_email_otp: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct UserTokenData {
    pub id: String,
    pub email: String,
    pub profile_image: Option<String>,
    pub email_verified: bool,
    pub password_set: bool,
    pub username: String,
    pub api_key: String,
    pub account_type: String,
    pub subscription: Option<Subscription>,
    pub default_storage_bytes: i64,
    pub country: Option<String>,
    pub bio: Option<String>,
}

pub fn token_data(user: User, subscription: Option<Subscription>) -> UserTokenData {
    UserTokenData {
        id: user.id,
        email: user.email,
        profile_image: user.profile_image,
        email_verified: user.email_verified,
        // Google accounts are created without a password (empty hash); the UI
        // uses this to offer "set password" vs "change password".
        password_set: !user.password_hash.trim().is_empty(),
        username: user.username,
        api_key: user.api_key,
        subscription: subscription,
        account_type: user.account_type,
        default_storage_bytes: user.default_storage_bytes,
        country: user.country,
        bio: user.bio,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub additional_space: i64,
    pub created_by: String,
    pub created_on: DateTime<Utc>,
    pub expires_on: DateTime<Utc>,
    pub invited: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct File {
    pub id: String,
    pub user_id: Option<String>,
    pub name: String,
    pub mime: String,
    pub size: i64,
    pub encrypted: bool,
    pub downloads: i64,
    pub created_on: DateTime<Utc>,
    pub total_chunks: i64,
    pub uploaded_chunks: i64,
    pub deleted: bool,
    pub sha256_checksum: String,
    pub blake3_checksum: String,
    pub public_access: bool,
    pub folder_id: Option<String>,
    pub owner_api_key: Option<String>,
    pub starred: bool,
    pub share_token: Option<String>,
    pub share_type: Option<String>,
    pub link_downloads: Option<i64>,
    pub link_max_downloads: Option<i64>,
    pub share_expires_at: Option<DateTime<Utc>>,
    pub share_password_hash: Option<String>,
    pub deleted_on: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub user_id: Option<String>,
    pub parent_id: Option<String>,
    pub uploaded_as_files: bool,
    pub created_on: DateTime<Utc>,
    pub updated_on: DateTime<Utc>,
    pub owner_api_key: Option<String>,
    pub starred: bool,
    pub share_token: Option<String>,
    pub share_type: Option<String>,
    pub link_downloads: Option<i64>,
    pub link_max_downloads: Option<i64>,
    pub share_expires_at: Option<DateTime<Utc>>,
    pub share_password_hash: Option<String>,
    pub deleted: bool,
    pub deleted_on: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct SupportTicket {
    pub id: String,
    pub user_id: Option<String>,
    pub username: String,
    pub email: String,
    pub category: String,
    pub subject: String,
    pub message: String,
    pub is_pro: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct SupportReply {
    pub id: String,
    pub ticket_id: String,
    pub author_role: String,
    pub author_name: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct Chunk {
    pub id: String,
    pub file_id: String,
    pub chunk_index: i32,
    pub size: i64,
    pub downloads: i64,
    pub size_on_server: i64,
    pub uploaded: bool,
    pub created_on: DateTime<Utc>,
    pub presigned_url: Option<String>,
    pub file_offset: i64,
    pub salt: Option<String>,
    pub nonce: Option<String>,
    pub checksum: String,
    pub uploading: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InviteCode {
    pub code: String,
    pub description: String,
    pub created_on: DateTime<Utc>,
    pub claimed_by: Option<String>,
    pub benefit: String,
    pub account_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct PromoCode {
    pub code: String,
    pub discount_percentage: i32,
    pub duration: String,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct Order {
    pub user_id: String,
    pub created_on: DateTime<Utc>,

    pub reference_id: String,
    pub payment_gateway: String,

    pub subscription_name: String,
    pub subscription_cycle: String,
    pub additional_space: i64,

    pub currency: String,
    pub amount: i64,
    pub status: String,

    pub details: JsonValue,
    pub transactions: Vec<JsonValue>,
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct AdminUser {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}
