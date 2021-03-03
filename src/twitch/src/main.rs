use std::sync::atomic::{AtomicBool, Ordering};
use twitchchat::PrivmsgExt;

mod config;
use config::*;

use clap::{App, Arg};

static IS_ADMIN_ONLY: AtomicBool = AtomicBool::new(false);

macro_rules! always_admin {
    ($privilege: ident, $chat: ident) => {
        if $privilege != Privilege::Admin {
            return $chat
                .writer
                .reply($chat.msg, "You do not have permission to run this command.")
                .unwrap();
        }
    };
}

macro_rules! sometimes_admin {
    ($privilege: ident, $chat: ident) => {
        if IS_ADMIN_ONLY.load(Ordering::Relaxed) == true && $privilege != Privilege::Admin {
            return $chat
                .writer
                .reply($chat.msg, "The bot is currently in admin-only mode.")
                .unwrap();
        }
    };
}

#[derive(PartialEq, Debug)]
enum Direction {
    Forward,
    Back,
    Left,
    Right,
    Up,
    Down,
}

fn dispatch_move(d: Direction) {
    println!("moving: {:?}", d);
}

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

    let bot = ChatBot::new_from_file(matches.value_of("config").unwrap())?
    .with_greeting("BooBot is online")
    .with_command(
        &["help"],
        |chat, _, _| {
            chat.writer.say(chat.msg, "Twitch doesn't support newlines in its commands so go read this for help: https://github.com/Shizcow/BooBot/blob/subproject/twitch/src/twitch/README.md").unwrap();
        },
    ).with_command(
        &["info"],
        move |chat, _, _| {
            let output = format!(
                "Uptime: {:.2?}, {}, other info coming soon!",
		start.elapsed(),
		if IS_ADMIN_ONLY.load(Ordering::Relaxed) {
		    "only admins can control"
		} else {
		    "all users can control"
		}
            );
            chat.writer.say(chat.msg, &output).unwrap();
        },
    ).with_command(
        &["source"],
        |chat, _, _| {
            chat.writer.say(chat.msg, "Source code: https://github.com/Shizcow/BooBot").unwrap();
        },
    ).with_command(
        &["q", "quit"],
        |chat, _, privilege| {
	    always_admin!(privilege, chat);
	    chat.writer.say(chat.msg, "Shutting down BooBot").unwrap();
	    smol::block_on(async move {
		chat.quit.notify().await
	    });
        },
    ).with_command(
        &["start"],
        |chat, _, privilege| {
	    always_admin!(privilege, chat);
	    match IS_ADMIN_ONLY.compare_exchange(true, false, Ordering::Acquire,
						 Ordering::Relaxed) {
		Ok(_) => chat.writer.say(chat.msg, "Bot is now available to all users").unwrap(),
		Err(_) => chat.writer.say(chat.msg, "Bot is already available to all users").unwrap(),
	    }
        },
    ).with_command(
        &["stop"],
        |chat, _, privilege| {
	    always_admin!(privilege, chat);
	    match IS_ADMIN_ONLY.compare_exchange(false, true, Ordering::Acquire,
						 Ordering::Relaxed) {
		Ok(_) => chat.writer.say(chat.msg, "Bot is now admin-only").unwrap(),
		Err(_) => chat.writer.say(chat.msg, "Bot is already admin-only").unwrap(),
	    }
        },
    ).with_command(
        &["f", "forward"],
        |chat, _, privilege| {
	    sometimes_admin!(privilege, chat);
	    dispatch_move(Direction::Forward);
        },
    );

    smol::block_on(async move { bot.run().await })
}
