extern crate pnet;
extern crate ipnetwork;
extern crate trust_dns_resolver;

use pnet::datalink::{self, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
// use pnet::transport::{self};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::{self/*, icmp*/};
use ipnetwork::IpNetwork;

use trust_dns_resolver::{Resolver, error as resolverError};

use std::net::{IpAddr, Ipv4Addr};
use std::string::String;
use std::sync::mpsc;
use std::thread;
use std::string;
use std::process;

use std::option::Option;

#[derive(Debug)]
pub struct Configuration {
    // dnsServers: Option<Vec<String>>
}

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
            // TODO: use configurable DNS servers
            let resolv = Resolver::from_system_conf().unwrap();
            return match resolv.lookup_ip(target.as_str()) {
                Err(_) => None,
                Ok(result) => result.iter().next()
            };
        }
    }
}

fn pick_network_interface_for_target(_: IpAddr) -> Option<NetworkInterface> {
    let name = "lo0"; // TODO: don't hardcode this :(
    datalink::interfaces()
        .into_iter()
        .filter(|iface: &NetworkInterface| iface.name == name)
        .next()
}

fn get_source_ip_from_iface(iface: NetworkInterface) -> Option<IpNetwork> {
    iface.ips
        .into_iter()
        .filter(|inet: &IpNetwork| inet.is_ipv4())
        .next()
}

fn mtr_fail(tx: mpsc::SyncSender<Event>, msg: String) {
    tx.send(Event{
        kind: EventKind::Error,
        data: Box::new(msg)
    });
}

fn unwrap_ipv4(ip: IpAddr) -> Ipv4Addr {
    match ip {
        IpAddr::V4(ip) => ip,
        _ => panic!("unwrap_ipv4")
    }
}

fn packetid() -> u16be {
    (process::id() & 0xFFFF) as u16be
}

impl MultiTracer {
    pub fn new(config: Configuration) -> MultiTracer {
        MultiTracer { config: config }
    }

    pub fn go(&self, target: String) -> mpsc::Receiver<Event> {
        let (ev_tx_orig, ev_rx) = mpsc::sync_channel(0);

        let ev_tx = ev_tx_orig.clone();
        thread::spawn(move || {
            let addr = match resolve_target(&target) {
                Some(ip) => ip,
                None => { mtr_fail(ev_tx, format!("invalid hostname: '{}'", target)); return; }
            };
            let iface = match pick_network_interface_for_target(addr) {
                Some(ifc) => ifc,
                None => { mtr_fail(ev_tx, "failed to init network interface".to_string()); return; }
            };

            let (mut tx, rx) = match datalink::channel(&iface, Default::default()) {
                Ok(Ethernet(tx, rx)) => (tx, rx),
                Ok(_) => { mtr_fail(ev_tx, "failed to init datalink channel, unknown type".to_string()); return; },
                Err(_) => { mtr_fail(ev_tx, "failed to init datalink channel, unknown error".to_string()); return; }
            };

            // sender
            let eve_tx_clone = ev_tx.clone();
            thread::spawn(move || {
                let mut packetdata: [u8; 28] = [0u8; 28];
                {
                    let mut packet = packet::ipv4::MutableIpv4Packet::new(&mut packetdata).expect("insufficient ipv4 packet length");
                    packet.set_version(4);
                    packet.set_header_length(5);
                    packet.set_total_length(28);
                    packet.set_ttl(57);
                    packet.set_identification(packetid());
                    packet.set_destination(unwrap_ipv4(addr));
                    match get_source_ip_from_iface(iface.clone()) {
                        Some(inet) => packet.set_source(unwrap_ipv4(inet.ip())),
                        _ => panic!("eek! no ipv4 source ip address found!")
                    };
                }

                println!("packet data: {:?}", &packetdata);
                println!("interface:   {:?}", &iface);

                match tx.send_to(&packetdata, None) {
                    Some(Ok(_)) => {
                        eve_tx_clone.send(Event{kind: EventKind::Ping, data: Box::new("sent!".to_string())})
                    },
                    _ => { mtr_fail(eve_tx_clone, "failed to send packet!".to_string()); return; }
                };
            });

            // receiver
            thread::spawn(move || {

            });

            // processor

            // ev_tx.send(Event{kind: EventKind::Ping, data: Box::new("hello!".to_string())}).unwrap();
        });

        return ev_rx;
    }
}