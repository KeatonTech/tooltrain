use serde::Deserialize;
use lazy_static::lazy_static;
use crate::{commander::base::types::{Column, PrimitiveValue, Value}, DataType, Primitive};

#[derive(Clone, Debug, Deserialize)]
pub struct Account {
    id: String,
    username: String,
    acct: String,
    display_name: String,
    #[serde(default)] locked: bool,
    #[serde(default)] bot: bool,
    discoverable: bool,
    created_at: String,
    note: String,
    url: String,
    avatar: String,
    avatar_static: String,
    header: String,
    header_static: String,
    #[serde(default)] followers_count: u32,
    #[serde(default)] following_count: u32,
    #[serde(default)] statuses_count: u32,
    last_status_at: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Status {
    id: String,
    created_at: String,
    url: String,
    replies_count: u32,
    reblogs_count: u32,
    favourites_count: u32,
    content: String,
    account: Account,
    text: Option<String>,
}


lazy_static! {
    pub static ref OUTPUT_TABLE_TYPE: DataType = DataType::TableType(vec![
        Column {
            name: "id".to_string(),
            description: "The ID of the status".to_string(),
            data_type: Primitive::StringType,
        },
        Column {
            name: "content".to_string(),
            description: "The content of the status".to_string(),
            data_type: Primitive::StringType,
        },
        Column {
            name: "created_at".to_string(),
            description: "The time the status was created".to_string(),
            data_type: Primitive::StringType,
        },
        Column {
            name: "account".to_string(),
            description: "The account that created the status".to_string(),
            data_type: Primitive::StringType,
        },
        Column {
            name: "likes_count".to_string(),
            description: "The number of likes on the status".to_string(),
            data_type: Primitive::NumberType,
        },
        Column {
            name: "replies_count".to_string(),
            description: "The number of replies to the status".to_string(),
            data_type: Primitive::NumberType,
        },
    ]);
}

impl Status {
    pub fn as_output_value(&self) -> Vec<PrimitiveValue> {
        vec![
            PrimitiveValue::StringValue(self.id.clone()),
            PrimitiveValue::StringValue(self.text.clone().unwrap_or_else(|| self.content.clone())),
            PrimitiveValue::StringValue(self.created_at.clone()),
            PrimitiveValue::StringValue(self.account.display_name.clone()),
            PrimitiveValue::NumberValue(self.favourites_count as f64),
            PrimitiveValue::NumberValue(self.replies_count as f64),
        ]
    }
}