use std::{
    io::{Read, Write},
    net::TcpStream,
};

#[derive(Debug, Clone)]
pub enum Packet {
    Message { channel: String, content: Vec<u8> },
    Subscribe { channel: String },
    SubscribeAll,
    Unsubscribe { channel: String },
    UnsubscribeAll,
}

#[derive(Debug, Clone, Copy)]
pub enum RefPacket<'a> {
    Message { channel: &'a str, content: &'a [u8] },
    Subscribe { channel: &'a str },
    SubscribeAll,
    Unsubscribe { channel: &'a str },
    UnsubscribeAll,
}

fn read_byte(stream: &mut TcpStream) -> Result<u8, Box<dyn std::error::Error>> {
    let mut buffer = [0u8];
    stream.read_exact(&mut buffer)?;
    Ok(buffer[0])
}

fn read_u32(stream: &mut TcpStream) -> Result<u32, Box<dyn std::error::Error>> {
    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_bytes(stream: &mut TcpStream, length: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut result = vec![0u8; length as usize];
    stream.read_exact(&mut result)?;
    Ok(result)
}

fn read_string(stream: &mut TcpStream) -> Result<String, Box<dyn std::error::Error>> {
    let length = read_u32(stream)?;
    Ok(String::from_utf8(read_bytes(stream, length)?)?)
}

fn write_byte(stream: &mut TcpStream, value: u8) -> Result<(), Box<dyn std::error::Error>> {
    stream.write_all(&[value])?;
    Ok(())
}

fn write_u32(stream: &mut TcpStream, value: u32) -> Result<(), Box<dyn std::error::Error>> {
    stream.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn write_string(stream: &mut TcpStream, str: &str) -> Result<(), Box<dyn std::error::Error>> {
    write_u32(stream, str.len() as u32)?;
    Ok(stream.write_all(str.as_bytes())?)
}

impl Packet {
    pub fn write(&self, stream: &mut TcpStream) -> Result<(), Box<dyn std::error::Error>> {
        self.as_ref_packet().write(stream)
    }

    pub fn read(stream: &mut TcpStream) -> Result<Self, Box<dyn std::error::Error>> {
        let id = read_byte(stream)?;
        match id {
            0 => {
                let channel = read_string(stream)?;
                let content_length = read_u32(stream)?;
                let content = read_bytes(stream, content_length)?;
                Ok(Self::Message { channel, content })
            }
            1 => {
                let channel = read_string(stream)?;
                Ok(Self::Subscribe { channel })
            }
            2 => Ok(Self::SubscribeAll),
            3 => {
                let channel = read_string(stream)?;
                Ok(Self::Unsubscribe { channel })
            }
            4 => Ok(Self::UnsubscribeAll),
            _ => panic!("Invalid packet id {}", id),
        }
    }

    pub fn as_ref_packet(&self) -> RefPacket<'_> {
        match self {
            Self::Message { channel, content } => RefPacket::Message { channel, content },
            Self::Subscribe { channel } => RefPacket::Subscribe { channel },
            Self::SubscribeAll => RefPacket::SubscribeAll,
            Self::Unsubscribe { channel } => RefPacket::Unsubscribe { channel },
            Self::UnsubscribeAll => RefPacket::UnsubscribeAll,
        }
    }
}

impl<'a> RefPacket<'a> {
    pub fn write(&self, stream: &mut TcpStream) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Self::Message { channel, content } => {
                write_byte(stream, 0)?;
                write_string(stream, channel)?;
                write_u32(stream, content.len() as u32)?;
                stream.write_all(content)?;
            }
            Self::Subscribe { channel } => {
                write_byte(stream, 1)?;
                write_string(stream, channel)?;
            }
            Self::SubscribeAll => write_byte(stream, 2)?,
            Self::Unsubscribe { channel } => {
                write_byte(stream, 3)?;
                write_string(stream, channel)?;
            }
            Self::UnsubscribeAll => write_byte(stream, 4)?,
        }
        Ok(())
    }
}
