use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};

use crate::packet::{Packet, RefPacket};

const LOOPBACK_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

pub struct ModstreamsClient {
    stream: TcpStream,
}

impl ModstreamsClient {
    pub fn new(port: u16) -> Self {
        Self::from_tcp_stream(TcpStream::connect(SocketAddr::new(LOOPBACK_ADDRESS, port)).unwrap())
    }

    pub fn from_tcp_stream(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub fn send(
        &mut self,
        channel: &str,
        content: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        RefPacket::Message { channel, content }.write(&mut self.stream)
    }

    pub fn subscribe(&mut self, channel: &str) -> Result<(), Box<dyn std::error::Error>> {
        RefPacket::Subscribe { channel }.write(&mut self.stream)
    }

    pub fn subscribe_all(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        RefPacket::SubscribeAll.write(&mut self.stream)
    }

    pub fn unsubscribe(&mut self, channel: &str) -> Result<(), Box<dyn std::error::Error>> {
        RefPacket::Unsubscribe { channel }.write(&mut self.stream)
    }

    pub fn unsubscribe_all(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        RefPacket::UnsubscribeAll.write(&mut self.stream)
    }

    pub fn read(&mut self) -> Result<Packet, Box<dyn std::error::Error>> {
        Packet::read(&mut self.stream)
    }

    pub fn try_clone(&self) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            stream: self.stream.try_clone()?,
        })
    }
}
