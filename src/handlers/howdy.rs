use async_trait::async_trait;
use matrix_sdk::ruma::events::AnyMessageEventContent;
use regex::Regex;
use tracing::{event, Level};

use super::{bot_mentioned, Handler};

#[derive(Debug, Clone)]
pub struct Howdy {
    re: Regex,
}

impl Howdy {
    pub fn new(_: matrix_sdk::Client) -> Self {
        Self {
            re: Regex::new(r"(?i)\b(hello|howdy|hi|oh hai)\b").unwrap(),
        }
    }
}

#[async_trait]
impl Handler for Howdy {
    fn cmd(&self) -> &str {
        "hello"
    }

    fn description(&self) -> &str {
        "Say hello! (Responds to other greetings, too)"
    }

    async fn handle(&self, sender: &str, message: &str) -> Option<AnyMessageEventContent> {
        if !self.re.is_match(message) {
            event!(Level::DEBUG, is_match = false);
            return None;
        }

        event!(Level::DEBUG, is_match = true);

        if !bot_mentioned(message) {
            // respond to greetings only some of the time, when not directed at us.
            if fastrand::f32() < 0.60 {
                return None;
            }
        }

        let responses = [
            "Hi!",
            "Howdy!",
            "Hello!",
            "HULLO?",
            &format!("Hi, {}!", sender),
            &format!("Howdy, {}!", sender),
            &format!("Hello, {}!", sender),
        ];

        let r = responses[fastrand::usize(..responses.len())];
        super::new_message(r.into())
    }
}

unsafe impl Sync for Howdy {}
unsafe impl Send for Howdy {}
