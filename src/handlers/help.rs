use async_trait::async_trait;
use matrix_sdk::ruma::events::AnyMessageEventContent;
use matrix_sdk::Client;
use regex::Regex;
use tracing::{event, Level};

use super::Handler;

#[derive(Debug, Clone)]
pub struct Help {
    client: Client,
    re: Regex,
}

impl Help {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            re: Regex::new(r"(?i)^(\s\*\s)?!help").unwrap(),
        }
    }
}

#[async_trait]
impl Handler for Help {
    fn cmd(&self) -> &str {
        "!help"
    }

    fn description(&self) -> &str {
        "Returns help information"
    }

    async fn handle(&self, _: &str, message: &str) -> Option<AnyMessageEventContent> {
        if !self.re.is_match(message) {
            event!(Level::DEBUG, is_match = false);
            return None;
        }

        event!(Level::DEBUG, is_match = true);

        let mut help = vec!["Here's a list of the things I respond to:".into()];
        for handler in super::get_handlers(&self.client, None) {
            if !handler.cmd().is_empty() {
                help.push(format!(
                    "* **{}** - {}",
                    handler.cmd(),
                    handler.description()
                ));
            }
        }

        super::new_message(help.join("\n"))
    }
}

unsafe impl Sync for Help {}
unsafe impl Send for Help {}
