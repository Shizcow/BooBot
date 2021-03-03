use twitchchat::PrivmsgExt;

mod config;
use config::*;

use clap::{App, Arg};

fn main() -> anyhow::Result<()> {
    let start = std::time::Instant::now();

    let matches = App::new("Twitch BooBot")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Takes a config file for connecting to twitch")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let bot = ChatBot::new_from_file(matches.value_of("config").unwrap())?.with_command(
        "help",
        |chat, _, _| {
            chat.writer.say(chat.msg, "Twitch doesn't support newlines in its commands so go read this for help: https://github.com/Shizcow/BooBot/blob/subproject/twitch/src/twitch/README.md").unwrap();
        },
    ).with_command(
        "info",
        move |chat, _, _| {
            let output = format!(
                "Uptime: {:.2?}, other info coming soon!",
		start.elapsed()
            );
            chat.writer.say(chat.msg, &output).unwrap();
        },
    ).with_command(
        "source",
        |chat, _, _| {
            chat.writer.say(chat.msg, "Source code: https://github.com/Shizcow/BooBot").unwrap();
        },
    ).with_command(
        "quit",
        |chat, _, privilege| {
	    if privilege != Privilege::Admin {
		chat.writer.reply(chat.msg, "This is an admin command. Run as ?quit if you are an admin.").unwrap();
	    } else {
		chat.writer.say(chat.msg, "Shutting down BooBot").unwrap();
		smol::block_on(async move {
		    chat.quit.notify().await
		});
	    }
        },
    );

    smol::block_on(async move { bot.run().await })
}
