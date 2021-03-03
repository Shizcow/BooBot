use anyhow::anyhow;
use serde::Deserialize;
use twitchchat::{connector, runner::AsyncRunner};

// This is the config file ingest format for the [connection] field
#[derive(Deserialize)]
struct Connection {
    channel: String,
    user: String,
    key: Option<String>,
    key_command: Option<String>,
}

// And there's only one [connection]
// This is really just to make serde happy
#[derive(Deserialize)]
struct Config {
    connection: Connection,
}

impl Config {
    // convert into a ChatBot by grabbing the key from the system
    fn resolve(self) -> anyhow::Result<ChatBot> {
        let key = if self.connection.key.is_some() {
            self.connection.key.unwrap()
        } else {
            self.get_key_password()?
        };
        Ok(ChatBot {
            channel: self.connection.channel,
            user: self.connection.user,
            key: key,
        })
    }
    // run key_command on system and return it's output
    fn get_key_password(&self) -> anyhow::Result<String> {
        let cmd_string = self.connection.key_command.as_ref().ok_or(anyhow!(
            "Neither `key` nor `key_command` field present in config file",
        ))?;
        Ok(String::from_utf8_lossy(
            &std::process::Command::new("sh")
                .args(&["-c", &cmd_string])
                .output()?
                .stdout,
        )
        .into_owned())
    }
}

// the useful struct -- here `key` is in plaintext
pub struct ChatBot {
    channel: String,
    user: String,
    key: String,
}

impl ChatBot {
    pub fn new_from_file(file: &str) -> anyhow::Result<Self> {
        toml::from_str::<Config>(&std::fs::read_to_string(file)?)?.resolve()
    }
    // generate the config required for twitchchat
    fn get_user_config(&self) -> anyhow::Result<twitchchat::UserConfig> {
        Ok(twitchchat::UserConfig::builder()
            .name(&self.user)
            .token(&self.key)
            .enable_all_capabilities()
            .build()?)
    }
    pub fn channel(&self) -> &String {
        &self.channel
    }
    pub async fn connect(&self) -> anyhow::Result<AsyncRunner> {
        // create a connector using ``smol``, this connects to Twitch.
        // you can provide a different address with `custom`
        // this can fail if DNS resolution cannot happen
        let connector = connector::smol::Connector::twitch()?;

        println!("we're connecting!");
        // create a new runner. this is a provided async 'main loop'
        // this method will block until you're ready
        let mut runner = AsyncRunner::connect(connector, &self.get_user_config()?).await?;
        println!("..and we're connected");

        // and the identity Twitch gave you
        println!("our identity: {:#?}", runner.identity);

        let channel = self.channel();

        println!("attempting to join '{}'", channel);
        let _ = runner.join(&channel).await?;
        println!("joined '{}'!", channel);

        Ok(runner)
    }
}
