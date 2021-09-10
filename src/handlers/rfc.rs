use async_trait::async_trait;
use matrix_sdk::ruma::events::AnyMessageEventContent;
use regex::Regex;
use tracing::{event, Level};

use super::Handler;

#[derive(Debug, Clone)]
pub struct Rfc {
    re: Regex,
}

impl Rfc {
    pub fn new(_: matrix_sdk::Client) -> Self {
        Self {
            re: Regex::new(r"(?i)^(\s\*\s)?!rfc:?\s+(?P<num>[0-9]+)$").unwrap(),
        }
    }
}

#[async_trait]
impl Handler for Rfc {
    fn cmd(&self) -> &str {
        "!rfc <number>"
    }

    fn description(&self) -> &str {
        "Generates a link to an RFC"
    }

    async fn handle(&self, _: &str, message: &str) -> Option<AnyMessageEventContent> {
        if !(message.starts_with("!rfc") || message.starts_with(" * !rfc")) {
            event!(Level::DEBUG, is_match = false);
            return None;
        }

        let responses = &[
            String::from("that's not an rfc, my dude"),
            String::from("what even is that because it's not an rfc"),
            String::from("no. just no"),
        ];
        let mut resp = responses[fastrand::usize(..responses.len())].clone();

        if let Some(captures) = self.re.captures(message) {
            if let Some(rfc) = captures.name("num") {
                event!(Level::DEBUG, is_match = true);
                resp = format!("https://tools.ietf.org/html/rfc{}", rfc.as_str());
            }
        }

        super::new_message(resp)
    }
}

unsafe impl Sync for Rfc {}
unsafe impl Send for Rfc {}
