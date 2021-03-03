use anyhow::anyhow;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Connection {
    channel: String,
    user: String,
    key: Option<String>,
    key_command: Option<String>,
}

#[derive(Deserialize)]
pub struct Config {
    connection: Connection,
}

impl Config {
    pub fn resolve(self) -> anyhow::Result<ConfigResolved> {
        let key = if self.connection.key.is_some() {
            self.connection.key.unwrap()
        } else {
            self.get_key_password()?
        };
        Ok(ConfigResolved {
            channel: self.connection.channel,
            user: self.connection.user,
            key: key,
        })
    }
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

pub struct ConfigResolved {
    channel: String,
    user: String,
    key: String,
}

impl ConfigResolved {
    pub fn get_user_config(&self) -> anyhow::Result<twitchchat::UserConfig> {
        // you need a `UserConfig` to connect to Twitch
        Ok(twitchchat::UserConfig::builder()
            // the name of the associated twitch account
            .name(&self.user)
            // and the provided OAuth token
           .token(&self.key)
            // and enable all of the advanced message signaling from Twitch
            .enable_all_capabilities()
            .build()?)
    }
    pub fn channels_to_join(&self) -> Vec<String> {
        self.channel.split(',').map(ToString::to_string).collect()
    }
}
