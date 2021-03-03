// NOTE: this demo requires `--feature smol`.
use twitchchat::{commands, connector, runner::AsyncRunner};

// this is a helper module to reduce code deduplication
mod include;
use crate::include::main_loop;

mod config;
use config::*;

async fn connect(config: &ConfigResolved) -> anyhow::Result<AsyncRunner> {
    // create a connector using ``smol``, this connects to Twitch.
    // you can provide a different address with `custom`
    // this can fail if DNS resolution cannot happen
    let connector = connector::smol::Connector::twitch()?;

    println!("we're connecting!");
    // create a new runner. this is a provided async 'main loop'
    // this method will block until you're ready
    let mut runner = AsyncRunner::connect(connector, &config.get_user_config()?).await?;
    println!("..and we're connected");

    // and the identity Twitch gave you
    println!("our identity: {:#?}", runner.identity);

    let channel = config.channel();

    println!("attempting to join '{}'", channel);
    let _ = runner.join(&channel).await?;
    println!("joined '{}'!", channel);

    Ok(runner)
}

use clap::{App, Arg};

fn main() -> anyhow::Result<()> {
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

    let config = toml::from_str::<Config>(&std::fs::read_to_string(
        matches.value_of("config").unwrap(),
    )?)?
    .resolve()?;

    let fut = async move {
        // connect and join the provided channels
        let runner = connect(&config).await?;

        // you can get a handle to shutdown the runner
        let quit_handle = runner.quit_handle();

        // you can get a clonable writer
        let mut writer = runner.writer();

        // spawn something off in the background that'll exit in 10 seconds
        smol::spawn({
            let mut writer = writer.clone();
            let channel = config.channel().clone();
            async move {
                println!("in 10 seconds we'll exit");
                smol::Timer::after(std::time::Duration::from_secs(10)).await;

                let cmd = commands::privmsg(&channel, "goodbye, world");
                writer.encode(cmd).await.unwrap();

                println!("sending quit signal");
                quit_handle.notify().await;
            }
        })
        .detach();

        // you can encode all sorts of 'commands'
        writer
            .encode(commands::privmsg(&config.channel(), "Test from a bot"))
            .await?;

        println!("starting main loop");
        // your 'main loop'. you'll just call next_message() until you're done
        main_loop(runner).await
    };

    smol::block_on(fut)
}
