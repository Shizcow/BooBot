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

    /* command overview:
    -f, forward: move forward
    -b, backward: move backward
    -l, left: turn left
    -r, right: turn right
    -u, up: look up
    -d, down: look down
    -say: emit a phrase from the built in speaker

    Always available:
    -help: display commands
    -info: bot state
    -up, uptime: uptime
    -source: source code

    Additionally, admin commands are available:
    -stop: stop accepting input from non-admins
    -start: begin accepting input from non-admins
    -quit: kill the bot
    */

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
    // .with_command("!info", |chat: Chat| {
    //     let output = format!("Source Code: https://github.com/Shizcow/BooBot");
    //     chat.writer.say(chat.msg, &output).unwrap();
    // })
    // .with_command("!hello", |chat: Chat| {
    //     let output = format!("hello {}!", chat.msg.name());
    //     // We can 'reply' to this message using a writer + our output message
    //     chat.writer.reply(chat.msg, &output).unwrap();
    // })
    // .with_command("!uptime", move |chat: Chat| {
    //     let output = format!("its been running for {:.2?}", start.elapsed());
    //     // We can send a message back (without quoting the sender) using a writer + our output message
    //     chat.writer.say(chat.msg, &output).unwrap();
    // })
    // .with_command("!quit", move |chat: Chat| {
    //     // because we're using sync stuff, turn async into sync with smol!
    //     smol::block_on(async move {
    //         // calling this will cause read_message() to eventually return Status::Quit
    //         chat.quit.notify().await
    //     });
    // });

    smol::block_on(async move { bot.run().await })
}
