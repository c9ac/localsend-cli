use crate::{Announce, DynError};
use miniserde::json;
use std::{
    collections::HashMap,
    io::ErrorKind::{TimedOut, WouldBlock},
    net::{Ipv4Addr, UdpSocket},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Device {
    pub info: Announce,
    pub address: String,
}

pub fn announce(device: &Announce) -> Result<(), DynError> {
    let multicast_addr = Ipv4Addr::new(224, 0, 0, 167);
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED)?;

    let announce = json::to_string(device).into_bytes();

    thread::spawn(move || {
        loop {
            let _ = socket.send_to(&announce, "224.0.0.167:53317");
            thread::sleep(Duration::from_secs(3));
        }
    });

    Ok(())
}

pub fn discover(timeout: Duration) -> Result<Vec<Device>, DynError> {
    let start = Instant::now();

    let multicast_addr = Ipv4Addr::new(224, 0, 0, 167);
    let socket = UdpSocket::bind("0.0.0.0:53317")?;
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    socket.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED)?;

    let mut filter = HashMap::new();

    while start.elapsed() < timeout {
        let mut buf = [0; 1024];
        match socket.recv_from(&mut buf) {
            Ok((n, src)) => {
                if let Ok(announce) =
                    json::from_str::<Announce>(&String::from_utf8_lossy(&buf[..n]))
                {
                    let src = src.to_string().split(":").next().unwrap().to_string();
                    filter.insert(announce.fingerprint.clone(), (announce, src));
                }
            }
            Err(e) if e.kind() == WouldBlock || e.kind() == TimedOut => continue,
            Err(e) => return Err(e.into()),
        };
    }

    if filter.is_empty() {
        return Err("No device was found".into());
    }

    let devices: Vec<Device> = filter
        .into_iter()
        .map(|(_, (announce, src))| Device {
            info: announce,
            address: src,
        })
        .collect();

    Ok(devices)
}
