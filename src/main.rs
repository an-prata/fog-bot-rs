// Copyright (c) 2023 Evan Overman (https://an-prata.it). Licensed under the MIT License.
// See LICENSE file in repository root for complete license text.

use curl::easy::Easy;
use dns_lookup;
use fastping_rs::PingResult::{Idle, Receive};
use fastping_rs::Pinger;
use public_ip;
use serenity::model::prelude::PrivateChannel;
use serenity::{
    async_trait,
    framework::StandardFramework,
    http::Http,
    model::prelude::{Message, Ready, UserId},
    prelude::{Context, EventHandler, GatewayIntents},
    Client,
};
use std::{
    env,
    net::IpAddr,
    sync::mpsc::{self, SyncSender},
    time::Duration,
};
use tokio::{task, time::sleep};

struct Handler {
    tx: SyncSender<UserId>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }

    async fn message(&self, _: Context, message: Message) {
        if let Err(err) = self.tx.send(message.author.id) {
            println!("Error getting message author's id: {}", err);
        }

        if !message.author.bot {
            println!("got message from {} - subscribing", message.author.name);
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let token = env::var("DISCORD_TOKEN")
        .expect("Expected discord token to be in the `DISCORD_TOKEN` enviornment variable");
    let intents = GatewayIntents::DIRECT_MESSAGES | GatewayIntents::GUILD_MESSAGES;
    let framework = StandardFramework::new();
    let (tx, rx) = mpsc::sync_channel(0);
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler { tx })
        .framework(framework)
        .await
        .expect("Error instantiating client");

    let mut user_ids: Vec<UserId> = Vec::new();
    let mut user_dms: Vec<PrivateChannel> = Vec::new();
    let mut curl = Easy::new();
    let http = Http::new(&token);

    curl.url("https://an-prata.it/")
        .expect("expected to be able to set url on curl");
    curl.write_function(|data| Ok(data.len()))
        .expect("expected to be able to set write function on curl");

    task::spawn(async move {
        if let Err(err) = client.start().await {
            println!("Client error: {:?}", err);
        }
    });

    loop {
        let ips: Vec<IpAddr> = match dns_lookup::lookup_host("an-prata.it") {
            Ok(ips) => ips,
            Err(err) => {
                println!("failed to resolve hostname: {}", err);
                continue;
            }
        };

        let (pinger, results) = match Pinger::new(None, None) {
            Ok(pair) => pair,
            Err(err) => {
                println!("failed to create pinger with error: {}", err);
                continue;
            }
        };

        for ip in ips {
            pinger.add_ipaddr(ip.to_string().as_str());
        }

        pinger.ping_once();

        while let Ok(id) = rx.try_recv() {
            if user_ids.iter().all(|i| *i != id) {
                user_ids.push(id);
            }
        }

        for id in user_ids.clone().iter() {
            let channel = match id.create_dm_channel(&http).await {
                Ok(c) => {
                    println!("open new direct message");
                    user_dms.push(c.clone());

                    if let Err(err) = c.say(&http, "opening dms with you!").await {
                        println!("could not open deirect message channel: {}", err);
                    }

                    c
                }
                Err(err) => {
                    println!("could not open direct message channel: {}", err);
                    continue;
                }
            };

            match results.recv() {
                Ok(res) => match res {
                    Idle { addr } => {
                        println!("the fog idle at address: {}", addr);

                        if let Err(err) = channel
                            .say(&http, format!("the fog idle at address: {}", addr))
                            .await
                        {
                            println!("failed to send direct message with error: {}", err);
                        }
                    }
                    Receive { addr, rtt } => {
                        println!("received from address {} in {:?}", addr, rtt);

                        if let Some(public_ip) = public_ip::addr().await {
                            if public_ip != addr {
                                println!(
                                    "adresses to dont match! (pinged: {}, expected: {})",
                                    addr, public_ip
                                );

                                if let Err(err) = channel
                                    .say(&http, format!("the fog's address does not match DNS!: (pinged: `{}`, expected: `{}`)", addr, public_ip))
                                    .await
                                {
                                    println!(
                                        "failed to send direct message with error: {}",
                                        err
                                    );
                                }
                            }
                        }
                    }
                },
                Err(err) => {
                    println!("error pinging: {}", err);

                    if let Err(err) = channel
                        .say(&http, format!("failed to ping the fog: {}", err))
                        .await
                    {
                        println!("failed to send direct message with error: {}", err);
                    }
                }
            }

            match curl.perform() {
                Ok(_) => (),
                Err(err) => {
                    println!("failed to perform http request: {}", err);

                    if let Err(err) = channel
                        .say(&http, format!("failed to perform http request: `{}`", err))
                        .await
                    {
                        println!("failed to send direct message with error: {}", err);
                    }
                }
            }

            match curl.response_code() {
                Ok(c) if c != 200 => {
                    println!("bad response over http: {}", c);

                    if let Err(err) = channel
                        .say(&http, format!("bad request over http: `{}`", c))
                        .await
                    {
                        println!("failed to send direct message with error: {}", err);
                    }
                }
                Ok(c) => {
                    println!("good response over http: {}", c);
                }
                Err(err) => {
                    println!("failed to get response code over http: {}", err);

                    if let Err(err) = channel
                        .say(
                            &http,
                            format!("failed to get response over http: `{}`", err),
                        )
                        .await
                    {
                        println!("failed to send direct message with error: {}", err);
                    }
                }
            }
        }

        sleep(Duration::from_secs(10)).await;
    }
}
