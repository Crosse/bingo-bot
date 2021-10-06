use std::collections::HashMap;

use async_trait::async_trait;
use matrix_sdk::ruma::events::room::message::{
    MessageEventContent, MessageType, TextMessageEventContent,
};
use matrix_sdk::ruma::events::AnyMessageEventContent;
use matrix_sdk::Client;
use regex::Regex;

use super::DISPLAY_NAME;

mod giphy;
mod help;
mod howdy;
mod python;
mod rfc;
mod troutslap;

use giphy::Giphy;
use help::Help;
use howdy::Howdy;
use python::KyleHatesPython;
use rfc::Rfc;
use troutslap::TroutSlap;

pub struct HelpInfo<'a> {
    pub command_name: &'a str,
    pub description: &'a str,
}

#[async_trait]
pub trait Handler: Send + Sync + std::fmt::Debug {
    fn cmd(&self) -> &str;
    fn description(&self) -> &str;
    async fn handle(&self, sender: &str, message: &str) -> Option<AnyMessageEventContent>;
}

pub fn get_handlers(
    client: &Client,
    config: Option<&HashMap<String, String>>,
) -> Vec<Box<dyn Handler>> {
    vec![
        Box::new(Help::new(client.clone())),
        Box::new(Giphy::new(client.clone(), config)),
        Box::new(Howdy::new(client.clone())),
        Box::new(KyleHatesPython::new(client.clone())),
        Box::new(Rfc::new(client.clone())),
        Box::new(TroutSlap::new(client.clone())),
    ]
}

pub(crate) fn bot_mentioned(message: &str) -> bool {
    Regex::new(&format!(r"(?i)\b{}\b", DISPLAY_NAME))
        .unwrap()
        .is_match(message)
}

pub(crate) fn new_message(message: String) -> Option<AnyMessageEventContent> {
    Some(AnyMessageEventContent::RoomMessage(
        MessageEventContent::new(MessageType::Text(TextMessageEventContent::markdown(
            message,
        ))),
    ))
}
