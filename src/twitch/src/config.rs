use anyhow::anyhow;
use serde::Deserialize;
use twitchchat::{
    commands,
    messages::{Commands, Privmsg},
    runner::{AsyncRunner, NotifyHandle, Status},
};

use std::collections::HashMap;

#[derive(PartialEq)]
pub enum Privilege {
    User,
    Admin,
}

// This is the config file ingest format for the [connection] field
#[derive(Deserialize)]
struct Connection {
    channel: String,
    user: String,
    key: Option<String>,
    key_command: Option<String>,
}

// what users have additional rights?
#[derive(Deserialize)]
struct Permissions {
    admins: Vec<String>,
}

// This handles actually streaming video
#[derive(Deserialize)]
struct Stream {
    command: String,
    rtmp: Option<String>,
    rtmp_command: Option<String>,
}

// And there's only one [connection]
// This is really just to make serde happy
#[derive(Deserialize)]
struct Config {
    connection: Connection,
    permissions: Permissions,
    stream: Stream,
}

impl Config {
    // convert into a ChatBot by grabbing the key from the system
    fn resolve(self) -> anyhow::Result<ChatBot> {
        let key = if self.connection.key.is_some() {
            self.connection.key.clone().unwrap()
        } else {
            self.get_key_password()?
        };

        let rtmp = self.get_expanded_rtmp();
        let stream_start_command = self.stream.command.replace("%s", &rtmp);
        Ok(ChatBot {
            stream_start_command,
            channel: self.connection.channel,
            user: self.connection.user,
            key: key,
            commands: HashMap::new(),
            admins: self.permissions.admins,
            greeting: None,
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
    fn get_expanded_rtmp(&self) -> String {
        self.stream
            .rtmp_command
            .as_ref()
            .map(|c| {
                String::from_utf8_lossy(
                    &std::process::Command::new("sh")
                        .args(&["-c", &c])
                        .output()
                        .expect("Shell command failed for rtmp_command")
                        .stdout,
                )
                .to_string()
            })
            .unwrap_or_else(|| {
                self.stream
                    .rtmp
                    .clone()
                    .expect("[stream] requires rtmp or rtmp_command")
            })
    }
}

// the useful struct -- here `key` is in plaintext
pub struct ChatBot {
    greeting: Option<&'static str>,
    channel: String,
    user: String,
    key: String,
    stream_start_command: String,
    commands: HashMap<
        &'static [&'static str],
        Box<dyn Fn(Chat<'_, '_>, Vec<&str>, Privilege) + Send + Sync + 'static>,
    >,
    admins: Vec<String>,
}

impl ChatBot {
    pub fn with_video_streaming(self) -> Self {
        std::process::Command::new("sh")
            .args(&["-c", &self.stream_start_command])
            .spawn()
            .expect("Failed to start stream"); // have fun
        self
    }
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
    pub fn with_greeting(mut self, g: &'static str) -> Self {
        self.greeting = Some(g);
        self
    }
    pub fn with_command(
        mut self,
        name: &'static [&'static str],
        cmd: impl Fn(Chat<'_, '_>, Vec<&str>, Privilege) + Send + Sync + 'static,
    ) -> Self {
        self.commands.insert(name, Box::new(cmd));
        self
    }
    pub async fn run(&self) -> anyhow::Result<()> {
        let connector = twitchchat::connector::smol::Connector::twitch()?;

        let mut runner = AsyncRunner::connect(connector, &self.get_user_config()?).await?;

        println!("connecting, we are: {}", runner.identity.username());

        println!("joining: {}", self.channel);
        if let Err(err) = runner.join(&self.channel).await {
            eprintln!("error while joining '{}': {}", self.channel, err);
        }

        if self.greeting.is_some() {
            runner
                .writer()
                .encode(commands::privmsg(&self.channel, self.greeting.unwrap()))
                .await?;
        }

        println!("Connected!");
        self.main_loop(&mut runner).await
    }

    // executes commands
    async fn main_loop(&self, runner: &mut AsyncRunner) -> anyhow::Result<()> {
        // this is clonable, but we can just share it via &mut
        // this is rate-limited writer
        let mut writer = runner.writer();
        // this is clonable, but using it consumes it.
        // this is used to 'quit' the main loop
        let quit = runner.quit_handle();

        loop {
            // this drives the internal state of the crate
            match runner.next_message().await? {
                // if we get a Privmsg (you'll get an Commands enum for all messages received)
                Status::Message(Commands::Privmsg(pm)) => {
                    // see if its a command and do stuff with it
                    if let Some((cmd, args)) = Self::parse_command(pm.data()) {
                        if let Some(command) = self.commands.iter().find_map(|(key, val)| {
                            key.iter().find(|alias| alias == &&cmd).map(|_| val)
                        }) {
                            let chat = Chat {
                                msg: &pm,
                                writer: &mut writer,
                                quit: quit.clone(),
                            };

                            // first need to check permissions
                            let user_privilege =
                                if self.admins.iter().find(|u| u == &&pm.name()).is_none() {
                                    Privilege::User
                                } else {
                                    Privilege::Admin
                                };

                            command(chat, args, user_privilege);
                        }
                    }
                }
                // stop if we're stopping
                Status::Quit | Status::Eof => break,
                // ignore the rest
                Status::Message(..) => continue,
            }
        }

        println!("Bot exited gracefully");
        Ok(())
    }

    fn parse_command(input: &str) -> Option<(&str, Vec<&str>)> {
        if input.chars().nth(0) != Some('!') {
            return None;
        }
        let mut i = input[1..].split(' ');
        let command = i.next()?;
        let args = i.collect();
        Some((command, args))
    }
}

pub struct Chat<'a, 'b: 'a> {
    pub msg: &'a Privmsg<'b>,
    pub writer: &'a mut twitchchat::Writer,
    pub quit: NotifyHandle,
}
