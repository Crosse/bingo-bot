use async_trait::async_trait;
use matrix_sdk::ruma::events::AnyMessageEventContent;
use regex::Regex;
use tracing::{event, Level};

use super::{bot_mentioned, Handler};

#[derive(Debug, Clone)]
pub struct TroutSlap {
    re: Regex,
}

impl TroutSlap {
    pub fn new(_: matrix_sdk::Client) -> Self {
        Self {
            re: Regex::new(r"(?i)^(\s\*\s)?!slap\s+(?P<name>.+)$").unwrap(),
        }
    }
}

#[async_trait]
impl Handler for TroutSlap {
    fn cmd(&self) -> &str {
        "!slap"
    }

    fn description(&self) -> &str {
        "a good ol' trout slapping"
    }

    async fn handle(&self, sender: &str, message: &str) -> Option<AnyMessageEventContent> {
        let captures = self.re.captures(message);
        if captures.is_none() {
            event!(Level::DEBUG, is_match = false);
            return None;
        }
        event!(Level::DEBUG, is_match = true);

        if bot_mentioned(message) {
            return super::new_message("EXCUSE ME I DON'T THINK SO".into());
        }

        let slapped = captures.unwrap().name("name").unwrap();
        super::new_message(format!(
            "_{} slaps {} around with a large trout_",
            sender,
            slapped.as_str()
        ))
    }
}

unsafe impl Sync for TroutSlap {}
unsafe impl Send for TroutSlap {}
