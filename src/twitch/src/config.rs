use anyhow::anyhow;
use serde::Deserialize;

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
pub struct Config {
    connection: Connection,
}

impl Config {
    // convert into a ConfigResolved by grabbing the key from the system
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
pub struct ConfigResolved {
    channel: String,
    user: String,
    key: String,
}

impl ConfigResolved {
    // generate the config required for twitchchat
    pub fn get_user_config(&self) -> anyhow::Result<twitchchat::UserConfig> {
        Ok(twitchchat::UserConfig::builder()
            .name(&self.user)
            .token(&self.key)
            .enable_all_capabilities()
            .build()?)
    }
    pub fn channel(&self) -> &String {
        &self.channel
    }
}
