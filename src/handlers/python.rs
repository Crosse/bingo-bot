use async_trait::async_trait;
use matrix_sdk::ruma::events::AnyMessageEventContent;
use tracing::{event, Level};

use super::Handler;

#[derive(Debug, Clone)]
pub struct KyleHatesPython {}

impl KyleHatesPython {
    pub fn new(_: matrix_sdk::Client) -> Self {
        Self {}
    }
}

#[async_trait]
impl Handler for KyleHatesPython {
    fn cmd(&self) -> &str {
        ""
    }

    fn description(&self) -> &str {
        ""
    }

    async fn handle(&self, _sender: &str, message: &str) -> Option<AnyMessageEventContent> {
        if !message.to_lowercase().contains("python") {
            event!(Level::DEBUG, is_match = false);
            return None;
        }

        event!(Level::DEBUG, is_match = true);

        // respond to greetings only some of the time.
        if fastrand::f32() < 0.60 {
            return None;
        }

        let responses = [
            "\"Python sucks\" —Kyle",
            "_you hear a sound from across the reaches of the internet: it's Kyle, telling Python to screw itself_",
            "\"I hate Python\" —Kyle",
        ];

        let r = responses[fastrand::usize(..responses.len())];
        super::new_message(r.into())
    }
}

unsafe impl Sync for KyleHatesPython {}
unsafe impl Send for KyleHatesPython {}
