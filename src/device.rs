use crate::{Announce, DynError};
use miniserde::json;
use smol::{Timer, net::UdpSocket};
use smol_timeout::TimeoutExt;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Device {
    pub info: Announce,
    pub address: IpAddr,
}

pub async fn announce(device: &Announce) -> Result<(), DynError> {
    // Setup udp
    let multicast_addr = Ipv4Addr::new(224, 0, 0, 167);
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.join_multicast_v4(multicast_addr, Ipv4Addr::UNSPECIFIED)?;

    // Setup data
    let announce = json::to_string(device).into_bytes();

    // Multicast (unblocking)
    smol::spawn(async move {
        loop {
            let _ = socket.send_to(&announce, "224.0.0.167:53317").await;
            Timer::after(Duration::from_secs(1)).await;
        }
    })
    .detach();

    Ok(())
}

pub async fn discover(timeout: Duration) -> Result<Vec<Device>, DynError> {
    // Begin time
    let begin = Instant::now();

    // Setup udp multicast
    let multicast_addr = Ipv4Addr::new(224, 0, 0, 167);
    let socket = UdpSocket::bind("0.0.0.0:53317").await?;
    socket.join_multicast_v4(multicast_addr, Ipv4Addr::UNSPECIFIED)?;

    // Filter for duplicate equipment
    let mut filter = HashMap::new();

    // Receive
    let mut buf = [0; 1024];
    loop {
        let remaining = timeout.saturating_sub(begin.elapsed()); // Never overflowed
        if remaining.is_zero() {
            break;
        }

        if let Some(Ok((n, src))) = socket.recv_from(&mut buf).timeout(remaining).await
            // Only handle correct announcement
            && let Ok(announce) = json::from_str::<Announce>(&String::from_utf8_lossy(&buf[..n]))
        {
            filter.insert(announce.fingerprint.clone(), (announce, src.ip()));
        }
    }

    // Exit when no devices was found
    if filter.is_empty() {
        return Err("No device was found".into());
    }

    // Convert hashmap into vector
    let devices: Vec<Device> = filter
        .into_iter()
        .map(|(_, (announce, src))| Device {
            info: announce,
            address: src,
        })
        .collect();

    Ok(devices)
}
