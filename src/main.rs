extern crate core;

use std::env;
use core::{MultiTracer, Configuration, EventKind};

fn main() {
    let mtr = MultiTracer::new(Configuration{});

    let rx = mtr.go(env::args().nth(1).unwrap());

    loop {
        match rx.recv() {
            Ok(event) => {
                println!("{:?}", event);
            },
            Err(_) => {
                println!("closing shop!");
                break;
            }
        }
    }
}
