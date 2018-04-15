extern crate pnet;
extern crate ipnetwork;
extern crate trust_dns_resolver;
extern crate pnet_macros_support;
extern crate libc;

use pnet::datalink::{self, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
use pnet_macros_support::types::{u16be};
// use pnet::transport::{self};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::{self/*, icmp*/};
use ipnetwork::IpNetwork;

use trust_dns_resolver::Resolver;

use std::net::{IpAddr, Ipv4Addr};
use std::string::String;
use std::sync::mpsc;
use std::thread;

use std::option::Option;

const ICMP_DATA_SIZE: usize = 64;
const PACKET_SIZE: usize = ICMP_DATA_SIZE + 8 + 20;

#[derive(Debug)]
pub struct Configuration {
    // dnsServers: Option<Vec<String>>
}

pub struct MultiTracer;

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

fn get_source_ip_from_iface(iface: NetworkInterface) -> IpAddr {
    iface.ips
        .into_iter()
        .filter(|inet: &IpNetwork| inet.is_ipv4())
        .next()
        .expect("get_source_ip_from_iface1")
        .ip()
}

fn mtr_fail(tx: mpsc::SyncSender<Event>, msg: String) {
    tx.send(Event{
        kind: EventKind::Error,
        data: Box::new(msg)
    }).unwrap();
}

fn unwrap_ipv4(ip: IpAddr) -> Ipv4Addr {
    match ip {
        IpAddr::V4(ip) => ip,
        _ => panic!("unwrap_ipv4")
    }
}

fn getpid() -> u32 {
    unsafe { libc::getpid() as u32 }
}

fn packetid() -> u16be {
    (getpid() & 0xFFFF) as u16be
}

impl MultiTracer {
    pub fn new(_: Configuration) -> MultiTracer {
        MultiTracer {}
    }

    pub fn go(&self, _: String) -> mpsc::Receiver<Event> {
        let (ev_tx_orig, ev_rx) = mpsc::sync_channel(0);

        let ev_tx = ev_tx_orig.clone();
        thread::spawn(move || {
            // let addr = match resolve_target(&target) {
            //     Some(ip) => ip,
            //     None => { mtr_fail(ev_tx, format!("invalid hostname: '{}'", target)); return; }
            // };
            let iface = match pick_network_interface_for_target(IpAddr::from(Ipv4Addr::new(127, 0, 0, 1))) {
                Some(ifc) => ifc,
                None => { return mtr_fail(ev_tx, "failed to init network interface".to_string()); }
            };

            let (mut tx, _) = match datalink::channel(&iface, Default::default()) {
                Ok(Ethernet(tx, rx)) => (tx, rx),
                Ok(_) => { mtr_fail(ev_tx, "failed to init datalink channel, unknown type".to_string()); return; },
                Err(err) => { mtr_fail(ev_tx, format!("failed to init datalink channel, error: {:?}", err)); return; }
            };

            // sender
            let eve_tx_clone = ev_tx.clone();
            thread::spawn(move || {
                // let mut packetdata: [u8; PACKET_SIZE] = [0u8; PACKET_SIZE];
                // {
                //     let mut packet = packet::ipv4::MutableIpv4Packet::new(&mut packetdata).expect("insufficient ipv4 packet length");
                //     packet.set_version(4);
                //     packet.set_header_length(5);
                //     packet.set_total_length(PACKET_SIZE as u16);
                //     packet.set_identification(packetid());
                //     packet.set_ttl(57);
                //     packet.set_destination(Ipv4Addr::new(127, 0, 0, 1));
                //     packet.set_next_level_protocol(IpNextHeaderProtocols::Icmp);
                //     packet.set_source(unwrap_ipv4(get_source_ip_from_iface(iface.clone())));
                // }

                // println!("interface:   {:?}", &iface);

                match tx.build_and_send(1, PACKET_SIZE, &mut |mut pdata: &mut [u8]| {
                    {
                        let mut packet = packet::ipv4::MutableIpv4Packet::new(&mut pdata).expect("insufficient ipv4 packet length");
                        packet.set_version(4);
                        packet.set_header_length(5);
                        packet.set_total_length(PACKET_SIZE as u16);
                        packet.set_identification(packetid());
                        packet.set_ttl(57);
                        packet.set_destination(Ipv4Addr::new(127, 0, 0, 1));
                        packet.set_next_level_protocol(IpNextHeaderProtocols::Icmp);
                        packet.set_source(unwrap_ipv4(get_source_ip_from_iface(iface.clone())));
                        {
                            let _a = pdata;
                            packet.set_checksum(packet::ipv4::checksum(&packet::ipv4::Ipv4Packet::new(&_a).expect("aa")));                            
                        }
                    }
                    {
                        let mut icmp_payload = packet::icmp::MutableIcmpPacket::new(&mut pdata[20..(PACKET_SIZE)]).expect("insufficient icmp packet length");
                        icmp_payload.set_icmp_type(packet::icmp::IcmpTypes::EchoRequest);
                        let mut randdata: [u8; ICMP_DATA_SIZE] = [0u8; ICMP_DATA_SIZE];
                        for i in 0..ICMP_DATA_SIZE {
                            randdata[i] = i as u8;
                        }
                        icmp_payload.set_payload(&randdata);
                    }
                    println!("packet data: {:?}", pdata.to_vec());
                }) {
                    Some(Ok(_)) => {
                        eve_tx_clone.send(Event{kind: EventKind::Ping, data: Box::new("sent!".to_string())}).unwrap();
                    },
                    _ => { mtr_fail(eve_tx_clone, "failed to send packet!".to_string()); return; }
                };
            });

            // receiver
            // thread::spawn(move || {

            // });

            // processor

            // ev_tx.send(Event{kind: EventKind::Ping, data: Box::new("hello!".to_string())}).unwrap();
        });

        return ev_rx;
    }
}