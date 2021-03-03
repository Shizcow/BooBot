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

    let bot = ChatBot::new_from_file(matches.value_of("config").unwrap())?
        .with_command("!help", |args: Args| {
            let output = format!(
                "Read the source code for help. A real help command will be implemented later."
            );
            args.writer.say(args.msg, &output).unwrap();
        })
        .with_command("!info", |args: Args| {
            let output = format!("Source Code: https://github.com/Shizcow/BooBot");
            args.writer.say(args.msg, &output).unwrap();
        })
        .with_command("!hello", |args: Args| {
            let output = format!("hello {}!", args.msg.name());
            // We can 'reply' to this message using a writer + our output message
            args.writer.reply(args.msg, &output).unwrap();
        })
        .with_command("!uptime", move |args: Args| {
            let output = format!("its been running for {:.2?}", start.elapsed());
            // We can send a message back (without quoting the sender) using a writer + our output message
            args.writer.say(args.msg, &output).unwrap();
        })
        .with_command("!quit", move |args: Args| {
            // because we're using sync stuff, turn async into sync with smol!
            smol::block_on(async move {
                // calling this will cause read_message() to eventually return Status::Quit
                args.quit.notify().await
            });
        });

    smol::block_on(async move { bot.run().await })
}
