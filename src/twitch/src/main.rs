use std::sync::atomic::{AtomicBool, Ordering};
use twitchchat::PrivmsgExt;

mod config;
use config::*;

use clap::{App, Arg};

use std::sync::Mutex;

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

use rppal::gpio::{Gpio, OutputPin};

struct MotorGPIOs {
    left_hbridge_left: OutputPin,
    left_hbridge_right: OutputPin,
    right_hbridge_left: OutputPin,
    right_hbridge_right: OutputPin,
}

impl MotorGPIOs {
    fn new(
        left_hbridge_left: u8,
        left_hbridge_right: u8,
        right_hbridge_left: u8,
        right_hbridge_right: u8,
    ) -> Result<Self, rppal::gpio::Error> {
        Ok(MotorGPIOs {
            left_hbridge_left: Gpio::new()?.get(left_hbridge_left)?.into_output(),
            left_hbridge_right: Gpio::new()?.get(left_hbridge_right)?.into_output(),
            right_hbridge_left: Gpio::new()?.get(right_hbridge_left)?.into_output(),
            right_hbridge_right: Gpio::new()?.get(right_hbridge_right)?.into_output(),
        })
    }

    fn dispatch_move(&mut self, d: Direction, forever: bool) {
        println!("moving: {:?}", d);

        self.left_hbridge_left.set_low();
        self.left_hbridge_right.set_low();
        self.right_hbridge_left.set_low();
        self.right_hbridge_right.set_low();

        std::thread::sleep(std::time::Duration::from_millis(10)); // debounce

        if d == Direction::Stop {
            return;
        }

        match &d {
            Direction::Forward => [&mut self.left_hbridge_left, &mut self.right_hbridge_left],
            Direction::Backward => [&mut self.left_hbridge_right, &mut self.right_hbridge_right],
            Direction::Left => [&mut self.left_hbridge_right, &mut self.right_hbridge_left],
            Direction::Right => [&mut self.left_hbridge_left, &mut self.right_hbridge_right],
            Direction::Stop => unimplemented!(),
        }
        .iter_mut()
        .for_each(|p| {
            p.set_high();
        });

        if !forever {
            std::thread::sleep(std::time::Duration::from_millis(match d {
                Direction::Forward | Direction::Backward => 1000,
                Direction::Left | Direction::Right => 300,
                Direction::Stop => unimplemented!(),
            }));

            self.left_hbridge_left.set_low();
            self.left_hbridge_right.set_low();
            self.right_hbridge_left.set_low();
            self.right_hbridge_right.set_low();
        }
    }
}

lazy_static::lazy_static! {
    static ref MGPIO: Mutex<MotorGPIOs> = Mutex::new(MotorGPIOs::new(17, 25, 8, 7).expect("Could not bind to gpio!"));
}

#[derive(PartialEq, Debug)]
enum Direction {
    Forward,
    Backward,
    Left,
    Right,
    Stop,
}

fn setup_move() {}

fn main() -> anyhow::Result<()> {
    let start = std::time::Instant::now();

    setup_move();

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
        |chat, args, privilege| {
	    sometimes_admin!(privilege, chat);
	    MGPIO.lock().unwrap().dispatch_move(Direction::Forward, args.get(0) == Some(&"1"));
        },
    ).with_command(
        &["b", "backward"],
        |chat, args, privilege| {
	    sometimes_admin!(privilege, chat);
	    MGPIO.lock().unwrap().dispatch_move(Direction::Backward, args.get(0) == Some(&"1"));
        },
    ).with_command(
        &["l", "left"],
        |chat, args, privilege| {
	    sometimes_admin!(privilege, chat);
	    MGPIO.lock().unwrap().dispatch_move(Direction::Left, args.get(0) == Some(&"1"));
        },
    ).with_command(
        &["r", "right"],
        |chat, args, privilege| {
	    sometimes_admin!(privilege, chat);
	    MGPIO.lock().unwrap().dispatch_move(Direction::Right, args.get(0) == Some(&"1"));
        },
    ).with_command(
        &["s", "stop"],
        |chat, _, privilege| {
	    sometimes_admin!(privilege, chat);
	    MGPIO.lock().unwrap().dispatch_move(Direction::Stop, false);
        },
    ).with_command(
        &["say"],
        |chat, args, privilege| {
	    sometimes_admin!(privilege, chat);
	    let phrase: String = args.join(" ");
	    println!("saying phrase '{}'", phrase);
	    std::process::Command::new("flite")
                .args(&["-t", &phrase])
                .output().expect("Failed to launch flite -- it it installed?");
        },
    );

    smol::block_on(async move { bot.run().await })
}
