//! Implementation of UDP broadcast sender.

use log::debug;
use std::net::UdpSocket;

const LISTENING_ADDRESS: &str = "0.0.0.0";

fn main() {
    let udp: UdpSocket = UdpSocket::bind("0.0.0.0:12000")
        .expect("Failed to bind to address for sending/receiving messages");

    //The below (recv_from) is set to blocking
    let mut buf = [0; 20];
    let (number_of_bytes, src_addr) = udp.recv_from(&mut buf).expect("Didn't receive data");
    let filled_buf = &mut buf[..number_of_bytes];
    debug!("{:?}", filled_buf);
}
