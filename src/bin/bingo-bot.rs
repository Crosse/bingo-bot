use std::collections::HashMap;
use std::error::Error;

use bingo_bot::BingoBot;
use config::Config;
use directories::ProjectDirs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut settings = Config::default();
    settings
        .merge(config::File::with_name("bot"))
        .unwrap()
        .merge(config::Environment::with_prefix("BINGO"))
        .unwrap();

    let homeserver = settings.get::<String>("homeserver")?;
    let username = settings.get::<String>("username")?;
    let password = settings.get::<String>("password")?;

    let mut level_filter = "bingo_bot=info";
    if settings.get::<bool>("debug").unwrap_or(false) {
        level_filter = "bingo_bot=debug";
    }

    tracing_subscriber::fmt()
        .with_env_filter(level_filter)
        .init();

    let store_path = match ProjectDirs::from("org", "crosse", "bingobot") {
        Some(p) => p,
        None => {
            eprintln!("error locating directory for bot store data");
            std::process::exit(1);
        }
    };
    let data_dir = store_path.data_dir();
    std::fs::create_dir_all(data_dir)?;

    let table = settings.get_table("handlers").expect("no handlers config");
    let mut conf: HashMap<String, String> = HashMap::new();
    for (k, v) in table.iter() {
        conf.insert(
            k.into(),
            v.clone()
                .into_str()
                .expect("handler config values must be strings"),
        );
    }

    let mut bot = BingoBot::new(&homeserver, data_dir, Some(conf))?;
    bot.login_and_sync(&username, &password).await?;

    Ok(())
}
