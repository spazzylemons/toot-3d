use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub acct: String,
    pub url: String,
    pub display_name: String,
    pub note: String,
    pub avatar: String,
    pub avatar_static: String,
    pub header: String,
    pub header_static: String,
    pub locked: bool,
    pub fields: Vec<Field>,
    pub emojis: Vec<CustomEmoji>,
    pub bot: bool,
    pub group: bool,
    pub discoverable: Option<bool>,
    #[serde(rename = "noindex")]
    pub no_index: Option<bool>,
    pub moved: Option<Box<Account>>,
    pub suspended: Option<bool>,
    pub limited: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub last_status_at: Option<NaiveDate>,
    pub statuses_count: u64,
    pub followers_count: u64,
    pub following_count: u64,
    pub source: Option<AccountSource>,
    pub role: Option<Role>,
}

#[derive(Deserialize)]
pub struct AccountSource {
    pub note: String,
    pub fields: Vec<Field>,
    pub privacy: Visibility,
    pub sensitive: bool,
    pub language: Option<String>,
    pub follow_requests_count: u64,
}

#[derive(Deserialize)]
pub struct Application {
    pub name: String,
    pub website: Option<String>,
    pub vapid_key: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

#[derive(Deserialize)]
pub struct CustomEmoji {
    pub shortcode: String,
    pub url: String,
    pub static_url: String,
    pub visible_in_picker: bool,
    pub category: Option<String>,
}

#[derive(Deserialize)]
pub struct Field {
    pub name: String,
    pub value: String,
    pub verified_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct Filter {
    pub id: String,
    pub title: String,
    pub context: FilterContext,
    pub expires_at: Option<DateTime<Utc>>,
    pub filter_action: FilterAction,
    pub keywords: Vec<FilterKeyword>,
    pub statuses: Vec<FilterStatus>,
}

#[derive(Deserialize)]
pub enum FilterAction {
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "hide")]
    Hide,
}

#[derive(Deserialize)]
pub enum FilterContext {
    #[serde(rename = "home")]
    Home,
    #[serde(rename = "notifications")]
    Notifications,
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "thread")]
    Thread,
    #[serde(rename = "account")]
    Account,
}

#[derive(Deserialize)]
pub struct FilterKeyword {
    pub id: String,
    pub keyword: String,
    pub whole_word: bool,
}

#[derive(Deserialize)]
pub struct FilterResult {
    pub filter: Filter,
    pub keyword_matches: Option<Vec<String>>,
    pub status_matches: Option<String>,
}

#[derive(Deserialize)]
pub struct FilterStatus {
    pub id: String,
    pub status_id: String,
}

#[derive(Deserialize)]
pub struct MediaAttachment {
    pub id: String,
    #[serde(rename = "type")]
    pub media_type: MediaType,
    pub url: String,
    pub preview_url: String,
    pub remote_url: Option<String>,
    pub meta: Value,
    pub description: Option<String>,
    pub blurhash: String,
}

#[derive(Deserialize)]
pub enum MediaType {
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "gifv")]
    Gifv,
    #[serde(rename = "video")]
    Video,
    #[serde(rename = "audio")]
    Audio,
}

#[derive(Deserialize)]
pub struct Poll {
    pub id: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub expired: bool,
    pub multiple: bool,
    pub votes_count: u64,
    pub voters_count: Option<u64>,
    pub options: Vec<PollOption>,
    pub emojis: Vec<CustomEmoji>,
    pub voted: bool,
    pub own_votes: Vec<usize>,
}

#[derive(Deserialize)]
pub struct PollOption {
    pub title: String,
    pub votes_count: Option<u64>,
}

#[derive(Deserialize)]
pub struct PreviewCard {
    pub url: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "type")]
    pub card_type: PreviewCardType,
    pub author_name: String,
    pub author_url: String,
    pub provider_name: String,
    pub provider_url: String,
    pub html: String,
    pub width: u64,
    pub height: u64,
    pub image: Option<String>,
    pub embed_url: String,
    pub blurhash: Option<String>,
    pub history: Option<Vec<TrendsHistory>>,
}

#[derive(Deserialize)]
pub struct TrendsHistory {
    pub day: String,
    pub accounts: String,
    pub uses: String,
}

#[derive(Deserialize)]
pub enum PreviewCardType {
    #[serde(rename = "link")]
    Link,
    #[serde(rename = "photo")]
    Photo,
    #[serde(rename = "video")]
    Video,
    #[serde(rename = "rich")]
    Rich,
}

#[derive(Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub color: String,
    pub position: Option<u64>,
    pub permissions: String,
    pub highlighted: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct Status {
    pub id: String,
    pub uri: String,
    pub created_at: DateTime<Utc>,
    pub account: Account,
    pub content: String,
    pub visibility: Visibility,
    pub sensitive: bool,
    pub spoiler_text: String,
    pub media_attachments: Vec<MediaAttachment>,
    pub application: Option<StatusApplication>,
    pub mentions: Vec<StatusMention>,
    pub tags: Vec<StatusTag>,
    pub emojis: Vec<CustomEmoji>,
    pub reblogs_count: u64,
    pub favourites_count: u64,
    pub replies_count: u64,
    pub url: Option<String>,
    pub in_reply_to_id: Option<String>,
    pub in_reply_to_account_id: Option<String>,
    pub reblog: Option<Box<Status>>,
    pub poll: Option<Poll>,
    pub card: Option<PreviewCard>,
    pub language: Option<String>,
    pub text: Option<String>,
    pub edited_at: Option<DateTime<Utc>>,
    pub favourited: bool,
    pub reblogged: bool,
    pub muted: bool,
    pub bookmarked: bool,
    pub pinned: Option<bool>,
    pub filter: Option<Vec<FilterResult>>,
}

#[derive(Deserialize)]
pub struct StatusApplication {
    pub name: String,
    pub website: Option<String>,
}

#[derive(Deserialize)]
pub struct StatusMention {
    pub id: String,
    pub username: String,
    pub url: String,
    pub acct: String,
}

#[derive(Deserialize)]
pub struct StatusTag {
    pub name: String,
    pub url: String,
}

#[derive(Deserialize)]
pub struct Token {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
    pub created_at: u64,
}

#[derive(Deserialize)]
pub enum Visibility {
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "unlisted")]
    Unlisted,
    #[serde(rename = "private")]
    Private,
    #[serde(rename = "direct")]
    Direct,
}
