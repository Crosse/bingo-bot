use std::collections::HashMap;
use std::io::Cursor;
use std::str::FromStr;

use async_trait::async_trait;
use matrix_sdk::ruma::events::room::message::{
    ImageMessageEventContent, MessageEventContent, MessageType,
};
use matrix_sdk::ruma::events::AnyMessageEventContent;
use matrix_sdk::{ruma, Client};
use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tracing::{event, Level};
use url::Url;

use super::Handler;
use crate::errors::*;

const GIPHY_API: &str = "https://api.giphy.com/v1/gifs/translate";

fn get_url(api_key: &str, keywords: &str, sender: &str) -> Result<Url> {
    let random_id = format!("{:x}", Sha256::digest(sender.as_bytes()));
    Url::parse(&format!(
        "{}?api_key={}&s={}&random_id={}",
        GIPHY_API, api_key, keywords, random_id
    ))
    .map_err(Error::Url)
}

#[derive(Debug, Deserialize)]
struct Response {
    data: GifData,
}

#[derive(Debug, Deserialize)]
struct GifData {
    id: String,
    images: ImageData,
}

#[derive(Debug, Deserialize)]
struct ImageData {
    downsized: Image,
}

#[derive(Debug, Deserialize)]
struct Image {
    url: String,
    width: String,
    height: String,
    size: String,
}

#[derive(Debug, Clone)]
pub struct Giphy {
    client: Client,
    api_key: Option<String>,
    re: Regex,
}

impl Giphy {
    pub fn new(client: Client, config: Option<&HashMap<String, String>>) -> Self {
        let mut s = Self {
            client,
            api_key: None,
            re: Regex::new(r"(?i)^(\s\*\s)?!gi(f|phy):?\s+(?P<keywords>.+)$").unwrap(),
        };

        if let Some(Some(key)) = config.map(|c| c.get("giphy_api_key")) {
            s.api_key = Some(key.to_string());
        }
        s
    }
}

#[async_trait]
impl Handler for Giphy {
    fn cmd(&self) -> &str {
        "!giphy <keywords>"
    }

    fn description(&self) -> &str {
        "Finds a GIF relevant to your interests"
    }

    async fn handle(&self, sender: &str, message: &str) -> Option<AnyMessageEventContent> {
        let api_key = match self.api_key.as_ref() {
            Some(k) => k,
            None => {
                event!(
                    Level::WARN,
                    "Giphy handler can't run without 'giphy_api_key' in the
        config!"
                );
                return None;
            }
        };

        let captures = match self.re.captures(message) {
            Some(c) => c,
            None => {
                event!(Level::DEBUG, is_match = false);
                return None;
            }
        };

        let keywords = match captures.name("keywords") {
            Some(kw) => kw,
            None => {
                event!(Level::DEBUG, is_match = false);
                return None;
            }
        };

        event!(Level::DEBUG, is_match = true);

        let url = match get_url(api_key, keywords.as_str(), sender) {
            Ok(u) => u,
            Err(e) => {
                event!(Level::WARN, "failed to parse URL: {:?}", e);
                return None;
            }
        };

        let resp: reqwest::Response = match reqwest::get(url).await {
            Ok(r) => r,
            Err(e) => {
                event!(
                    Level::ERROR,
                    "error communicating with the GIPHY API: {:?}",
                    e
                );
                return None;
            }
        };
        let resp_json: Response = match resp.json().await {
            Ok(r) => r,
            Err(e) => {
                event!(Level::ERROR, "error deserializing GIPHY response: {:?}", e);
                return None;
            }
        };

        let gif = resp_json.data.images.downsized;
        event!(
            Level::DEBUG,
            "retrieved a {}x{} GIF ({}B), id: {}",
            gif.width,
            gif.height,
            gif.size,
            resp_json.data.id,
        );

        let gif_data = match reqwest::get(gif.url).await {
            Ok(d) => d,
            Err(e) => {
                event!(Level::ERROR, "error downloading GIF: {:?}", e);
                return None;
            }
        };

        let bytes = match gif_data.bytes().await {
            Ok(d) => d,
            Err(e) => {
                event!(
                    Level::ERROR,
                    "error getting GIF bytes from response: {:?}",
                    e
                );
                return None;
            }
        };

        let mut cursor = Cursor::new(bytes);

        let uploaded = match self.client.upload(&mime::IMAGE_GIF, &mut cursor).await {
            Ok(resp) => resp,
            Err(e) => {
                event!(Level::WARN, "error uploading image: {:?}", e);
                return None;
            }
        };

        let mut info = matrix_sdk::ruma::events::room::ImageInfo::new();
        info.height = Some(ruma::UInt::from_str(&gif.height).unwrap_or_default());
        info.width = Some(ruma::UInt::from_str(&gif.width).unwrap_or_default());
        info.mimetype = Some("image/gif".into());
        info.size = Some(ruma::UInt::from_str(&gif.size).unwrap_or_default());

        Some(AnyMessageEventContent::RoomMessage(
            MessageEventContent::new(MessageType::Image(ImageMessageEventContent::plain(
                format!("GIPHY id: {}", resp_json.data.id),
                uploaded.content_uri,
                Some(Box::new(info)),
            ))),
        ))
    }
}

unsafe impl Sync for Giphy {}
unsafe impl Send for Giphy {}
