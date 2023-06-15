// Copyright (c) 2023 Evan Overman (https://an-prata.it). Licensed under the MIT License.
// See LICENSE file in repository root for complete license text.

mod http_get;
mod ping;

use fastping_rs::PingResult::{Idle, Receive};
use http_get::HttpGetter;
use ping::SingleHost;
use public_ip;
use serenity::model::prelude::{PrivateChannel, Activity};
use serenity::model::user::User;
use serenity::{
    async_trait,
    framework::StandardFramework,
    http::Http,
    model::prelude::{Message, Ready},
    prelude::{Context, EventHandler, GatewayIntents},
    Client,
};
use std::{
    env,
    sync::mpsc::{self, SyncSender},
    time::Duration,
};
use tokio::{task, time::sleep};

const URL: &str = "https://an-prata.it/";
const HOST: &str = "an-prata.it";

struct Handler {
    tx: SyncSender<User>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!\n\n", ready.user.name);
        
        ctx.set_activity(Activity::watching(HOST)).await;
    }

    async fn message(&self, ctx: Context, message: Message) {
        if !message.author.bot && message.content.starts_with("!subscribe") {
            match message.reply(&ctx.http, "Got it! Check you DMs!").await {
                Ok(_) => (),
                Err(err) => println!("could not reply to message from {}: {}", message.author, err)
            };

            println!("got message from {} - subscribing", message.author.name);

            if let Err(err) = self.tx.send(message.author) {
                println!("Error getting message author: {}", err);
            }
        }
    }
}

fn is_same_err(a: &String, b: &String) -> bool {
    for (a, b) in a.chars().zip(b.chars()) {
        if a != b {
            return false;
        }

        if a == ':' {
            return true;
        }
    }

    false
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let token = env::var("DISCORD_TOKEN")
        .expect("Expected discord token to be in the `DISCORD_TOKEN` enviornment variable");
    let intents = GatewayIntents::DIRECT_MESSAGES | GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let framework = StandardFramework::new();
    let (tx, rx) = mpsc::sync_channel(0);
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler { tx })
        .framework(framework)
        .await
        .expect("Error instantiating client");

    let mut user_dms: Vec<PrivateChannel> = Vec::new();

    let mut web_getter = HttpGetter::new(URL).expect("expected to be able to setup curl");
    let http = Http::new(&token);

    task::spawn(async move {
        if let Err(err) = client.start().await {
            println!("Client error: {:?}", err);
        }
    });

    let mut msg = String::new();
    let mut prev_msg = String::new();

    loop {
        if msg.is_empty() && !prev_msg.is_empty(){
            msg = "the fog appears to be functioning normaly again! :D".to_string();
        } 
        
        if !is_same_err(&msg, &prev_msg) && !msg.is_empty() {
            for channel in user_dms.iter() {
                if let Err(err) = channel.say(&http, msg.clone()).await {
                    println!(
                        "failed to send direct message to {}: {}",
                        channel.recipient, err
                    );
                }
            }
        }

        if !msg.is_empty() {
            println!("{}\n", msg.clone());
        }

        prev_msg = msg.clone();
        msg = String::new();
        
        let single_host = match SingleHost::new(HOST) {
            Ok(h) => h,
            Err(err) => {
                msg.insert_str(
                    msg.len(),
                    format!("could not resolve hostname ({}): {}\n", HOST, err).as_str(),
                );
                continue;
            }
        };

        single_host.ping();

        while let Ok(user) = rx.try_recv() {
            if !user_dms.iter().any(|c| c.recipient == user) {
                match user.create_dm_channel(&http).await {
                    Ok(channel) => {
                        if let Err(err) = channel
                            .say(
                                &http, 
                                format!("Hello {}! I'll be opening DMs with you to notify you of server events!\n", user)
                            )
                            .await 
                        {
                            println!("failed to send direct to {} message with error: {}", user, err);
                        }

                        user_dms.push(channel)
                    }
                    Err(_) => {
                        println!("failed to open direct message channel with {}", user);
                        continue;
                    }
                }
            }
        }

        match single_host.results() {
            Ok(res) => match res {
                Idle { addr } => {
                    msg.insert_str(
                        msg.len(),
                        format!("the fog idle at address: {}\n", addr).as_str(),
                    );
                }
                Receive { addr, rtt } => {
                    println!("received from address {} in {:?}", addr, rtt);

                    if let Some(public_ip) = public_ip::addr().await {
                        if public_ip != addr {
                            msg.insert_str(
                                msg.len(),
                                format!("the fog's address does not match DNS!: (pinged: `{}`, expected: `{}`)\n", addr, public_ip).as_str(),
                            );
                        }
                    }
                }
            },
            Err(err) => {
                msg.insert_str(
                    msg.len(),
                    format!("failed to ping the fog: {}\n", err).as_str(),
                );
            }
        }

        match web_getter.run() {
            Ok(c) if c != 200 => {
                msg.insert_str(
                    msg.len(),
                    format!("bad responce over http: `{}`\n", c).as_str(),
                );
            }
            Ok(c) => {
                println!("good response over http: {}", c);
            }
            Err(err) => {
                msg.insert_str(
                    msg.len(),
                    format!("failed to perform http request: `{}`\n", err).as_str(),
                );
            }
        }

        sleep(Duration::from_secs(10)).await;
    }
}
