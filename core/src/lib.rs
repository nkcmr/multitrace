extern crate pnet;
extern crate trust_dns_resolver;

use pnet::datalink::{self, NetworkInterface};
use pnet::datalink::Channel::Ethernet;

use trust_dns_resolver::{Resolver, error as resolverError};

use std::net::{IpAddr, Ipv4Addr};
use std::string::String;
use std::sync::mpsc;
use std::thread;
use std::string;

use std::error::Error;
use std::option::Option;

#[derive(Debug)]
pub struct Configuration;

pub struct MultiTracer {
    config: Configuration
}

#[derive(PartialEq, Debug)]
pub enum EventKind {
    // Error is a type of event that signals that the process has fatal died of
    // and error.
    Error,

    Ping,
    Pong,
    Loss
}

#[derive(Debug)]
pub struct Event {
    pub kind: EventKind,
    pub data: Box<String>
}

fn resolve_target(target: &String) -> Option<IpAddr> {
    return match target.as_str().parse() {
        Ok(ip_addr) => Some(ip_addr),
        _ => {
            let resolv = Resolver::from_system_conf().unwrap();
            return match resolv.lookup_ip(target.as_str()) {
                Err(_) => None,
                Ok(result) => result.iter().next()
            };
        }
    }
}

fn pick_network_interface_for_target(_: IpAddr) -> Option<NetworkInterface> {
    let name = "en0"; // TODO: don't hardcode this :(
    datalink::interfaces()
        .into_iter()
        .filter(|iface: &NetworkInterface| iface.name == name)
        .next()
}

fn mtr_fail(tx: mpsc::SyncSender<Event>, msg: String) {
    tx.send(Event{
        kind: EventKind::Error,
        data: Box::new(msg)
    });
}

impl MultiTracer {
    pub fn new(config: Configuration) -> MultiTracer {
        MultiTracer { config: config }
    }

    pub fn go(&self, target: String) -> mpsc::Receiver<Event> {
        let (tx_orig, rx) = mpsc::sync_channel(0);

        let tx = tx_orig.clone();
        thread::spawn(move || {
            let addr = match resolve_target(&target) {
                Some(ip) => ip,
                None => { mtr_fail(tx, format!("invalid hostname: '{}'", target)); return; }
            };
            let iface = match pick_network_interface_for_target(addr) {
                Some(ifc) => ifc,
                None => { mtr_fail(tx, "failed to init network interface".to_string()); return; }
            };

            let (mut ch_tx, mut ch_rx) = match datalink::channel(&iface, Default::default()) {
                Ok(Ethernet(tx, rx)) => (tx, rx),
                Ok(_) => { tx.send(Event{kind: EventKind::Error, data: Box::new("unhandled network interface channel type".to_string())}).unwrap(); return; },
                Err(e) => { tx.send(Event{kind: EventKind::Error, data: Box::new(format!("error opening network interface channel: {}", e))}).unwrap(); return; }
            };

            // sender
            thread::spawn(move || {
                ch_tx.build_and_send(1, );
            });

            // receiver
            thread::spawn(move || {

            });

            // processor
            for 

            tx.send(Event{kind: EventKind::Ping, data: Box::new("hello!".to_string())}).unwrap();
        });

        return rx;
    }
}