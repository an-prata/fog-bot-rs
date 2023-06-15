// Copyright (c) 2023 Evan Overman (https://an-prata.it). Licensed under the MIT License.
// See LICENSE file in repository root for complete license text.

use std::{
    io,
    sync::mpsc::{Receiver, RecvError},
};

use fastping_rs::{PingResult, Pinger};

pub struct SingleHost {
    pinger: Pinger,
    results: Receiver<PingResult>,
}

impl SingleHost {
    /// Creates a new [`SingleHost`] from a host name `str`.
    ///
    /// [`SingleHost`]: SingleHost
    pub fn new(host: &str) -> io::Result<Self> {
        let ips = dns_lookup::lookup_host(host)?;
        let (pinger, results) = match Pinger::new(None, None) {
            Ok(p) => p,
            Err(err) => return Err(io::Error::new(io::ErrorKind::Other, err)),
        };

        for ip in ips {
            pinger.add_ipaddr(ip.to_string().as_str());
        }

        Ok(Self { pinger, results })
    }

    /// Makes a single ping to the host.
    pub fn ping(&self) {
        self.pinger.ping_once();
    }

    /// Get results from last ping.
    pub fn results(&self) -> Result<PingResult, RecvError> {
        self.results.recv()
    }
}
