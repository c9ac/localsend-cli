use crate::message::Announce;
use miniserde::json;
use std::{
    error::Error,
    net::{Ipv4Addr, UdpSocket},
    thread::{sleep, spawn},
    time::Duration,
};

pub fn receive(alias: &str, port: usize) -> Result<(), Box<dyn Error>> {
    let device = Announce::build(alias, port);

    discover(&device)?;

    loop {
        sleep(Duration::from_secs(1));
    }
}

fn discover(device: &Announce) -> Result<(), Box<dyn Error>> {
    // Setup udp multicast
    let multicast_addr = Ipv4Addr::new(224, 0, 0, 167);
    let socket = UdpSocket::bind("0.0.0.0:53318")?;
    socket.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED)?;

    // Setup data
    let announce = json::to_string(device).into_bytes();

    // Multicast
    spawn(move || {
        loop {
            let _ = socket.send_to(&announce, "224.0.0.167:53317");
            sleep(Duration::from_secs(3));
        }
    });

    Ok(())
}
