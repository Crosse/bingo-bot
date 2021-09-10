use std::collections::HashMap;
use std::path::Path;

use matrix_sdk::{
    room::Room,
    ruma::events::{
        room::{
            member::MemberEventContent,
            message::{MessageEventContent, MessageType, TextMessageEventContent},
        },
        StrippedStateEvent, SyncMessageEvent,
    },
    Client, ClientConfig, SyncSettings,
};

use tokio::time::{sleep, Duration};
use tracing::{event, Level};
use url::Url;

pub(crate) mod errors;
pub use errors::*;

pub mod handlers;

static DISPLAY_NAME: &str = "Bingo";

#[derive(Debug)]
pub struct BingoBot {
    client: Client,
    config: Option<HashMap<String, String>>,
}

impl BingoBot {
    pub fn new(
        homeserver: &str,
        store_path: &Path,
        config: Option<HashMap<String, String>>,
    ) -> Result<Self> {
        let homeserver = Url::parse(homeserver)?;

        let sp = store_path.to_string_lossy().to_string();
        let client_config = ClientConfig::new().store_path(&sp);
        event!(Level::DEBUG, "store path: {}", &sp);

        let client = Client::new_with_config(homeserver, client_config)?;

        Ok(Self { client, config })
    }

    pub async fn login_and_sync(&mut self, username: &str, password: &str) -> Result<()> {
        self.login(username, password).await?;
        self.sync().await
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<()> {
        event!(Level::DEBUG, "attempting to log in as {}", username);
        self.client
            .login(username, password, None, Some(DISPLAY_NAME))
            .await?;
        event!(Level::INFO, "successfully logged in as {}", username);

        // XXX: is this needed?
        self.client.set_display_name(Some(DISPLAY_NAME)).await?;

        // throw away old messages
        event!(Level::DEBUG, "performing initial sync");
        self.client.sync_once(SyncSettings::default()).await?;
        event!(Level::DEBUG, "finished initial sync");

        let joined = self.client.joined_rooms();
        if !joined.is_empty() {
            let mut empty_rooms = vec![];
            let mut names = vec![];
            for room in joined {
                names.push(room_name_or_id(&Room::from(room.clone())).await);

                if let Ok(members) = room.active_members().await {
                    if members.len() <= 1 {
                        empty_rooms.push(room);
                    }
                }
            }
            event!(
                Level::INFO,
                "currently joined to {} rooms: {}",
                names.len(),
                names.join(", ")
            );

            if !empty_rooms.is_empty() {
                event!(
                    Level::INFO,
                    "leaving rooms where {} is the only member",
                    DISPLAY_NAME
                );
                for room in empty_rooms {
                    if room
                        .get_member(&self.client.user_id().await.unwrap())
                        .await?
                        .is_some()
                    {
                        room.leave().await?;
                        event!(
                            Level::INFO,
                            "left room \"{}\"",
                            room_name_or_id(&Room::from(room)).await
                        );
                    }
                }
            }
            self.client.sync_once(SyncSettings::default()).await?;
        }

        let invited = self.client.invited_rooms();
        if !invited.is_empty() {
            event!(Level::INFO, "joining rooms to which we were invited");
            for room in invited {
                let display_name = match room.display_name().await {
                    Ok(name) => format!(" ({})", name),
                    Err(_) => String::from(""),
                };

                event!(
                    Level::INFO,
                    "joining room {}{}",
                    room.room_id().as_str(),
                    display_name
                );
                match room.accept_invitation().await {
                    Ok(_) => {}
                    Err(e) => event!(Level::ERROR, "failed to join room: {}", e),
                }
            }
        }

        let hcli = self.client.clone();
        let config = self.config.clone();
        self.client
            .register_event_handler(move |ev, room, client| {
                Self::on_room_message(
                    ev,
                    room,
                    client,
                    handlers::get_handlers(&hcli, config.as_ref()),
                )
            })
            .await;

        self.client
            .register_event_handler(Self::on_stripped_state_member)
            .await;
        event!(Level::DEBUG, "registered event handlers");

        Ok(())
    }

    pub async fn sync(&self) -> Result<()> {
        event!(Level::DEBUG, "performing sync");
        let settings = SyncSettings::default().token(self.client.sync_token().await.unwrap());
        self.client.sync(settings).await;
        event!(Level::DEBUG, "sync finished");
        Ok(())
    }

    async fn on_room_message(
        event: SyncMessageEvent<MessageEventContent>,
        client: Client,
        room: Room,
        handlers: Vec<Box<dyn handlers::Handler>>,
    ) {
        if let Room::Joined(room) = room {
            if let SyncMessageEvent {
                content:
                    MessageEventContent {
                        msgtype: MessageType::Text(TextMessageEventContent { body: msg_body, .. }),
                        ..
                    },
                sender,
                ..
            } = event
            {
                let member = room.get_member(&sender).await.unwrap().unwrap();
                let room_name = room.name().unwrap_or_else(|| room.room_id().to_string());

                event!(
                    Level::INFO,
                    room = room_name.as_str(),
                    sender = member.user_id().as_str(),
                    msg = msg_body.as_str(),
                );

                if sender == client.user_id().await.unwrap() {
                    return;
                }

                let sender_name = member
                    .display_name()
                    .unwrap_or_else(|| member.user_id().as_str());

                for h in handlers {
                    if let Some(content) = h.handle(sender_name, &msg_body).await {
                        let typing = room.typing_notice(true).await.is_ok();

                        let millis = fastrand::u64(500..=1500);
                        sleep(Duration::from_millis(millis)).await;
                        room.send(content, None).await.unwrap();

                        if typing {
                            room.typing_notice(false).await.unwrap();
                        }
                        break;
                    }
                }
            }
        }
    }

    async fn on_stripped_state_member(
        room_member: StrippedStateEvent<MemberEventContent>,
        client: Client,
        room: Room,
    ) {
        if room_member.state_key != client.user_id().await.unwrap() {
            return;
        }

        if let Room::Invited(room) = room {
            let display_name = room.display_name().await;
            let name = match &display_name {
                Ok(n) => n,
                Err(_) => room.room_id().as_str(),
            };
            event!(Level::INFO, "auto-joining room {}", name);

            let mut delay = 2;

            while let Err(err) = room.accept_invitation().await {
                event!(
                    Level::ERROR,
                    "failed to join room {} ({:?}), retrying in {}s",
                    room.room_id(),
                    err,
                    delay
                );

                sleep(Duration::from_secs(delay)).await;
                delay *= 2;
                if delay > 3600 {
                    event!(
                        Level::ERROR,
                        "can't join room {} ({:?})",
                        room.room_id(),
                        err
                    );
                    break;
                }
            }
            event!(
                Level::ERROR,
                "successfully joined room {}",
                room.name().unwrap_or_else(|| room.room_id().to_string())
            );
        }
    }
}

async fn room_name_or_id(room: &Room) -> String {
    match room.display_name().await {
        Ok(name) => name,
        Err(_) => room.room_id().to_string(),
    }
}
