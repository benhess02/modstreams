use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    sync::mpsc,
    thread,
};

use modstreams_core::Packet;

const LOOPBACK_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

#[derive(Debug)]
enum SubscriptionSet {
    Include(HashSet<String>),
    Exclude(HashSet<String>),
}

impl SubscriptionSet {
    fn new() -> Self {
        Self::Include(HashSet::new())
    }

    fn subscribe(&mut self, channel: String) {
        match self {
            Self::Include(channels) => {
                channels.insert(channel);
            }
            Self::Exclude(channels) => {
                channels.remove(&channel);
            }
        }
    }

    fn unsubscribe(&mut self, channel: String) {
        match self {
            Self::Include(channels) => {
                channels.remove(&channel);
            }
            Self::Exclude(channels) => {
                channels.insert(channel);
            }
        }
    }

    fn subscribe_all(&mut self) {
        *self = Self::Exclude(HashSet::new());
    }

    fn unsubscribe_all(&mut self) {
        *self = Self::Include(HashSet::new());
    }

    fn is_subscribed(&self, channel: &str) -> bool {
        match self {
            Self::Include(channels) => channels.contains(channel),
            Self::Exclude(channels) => !channels.contains(channel),
        }
    }
}

#[derive(Debug)]
struct DaemonClient {
    stream: TcpStream,
    channels: SubscriptionSet,
}

#[derive(Debug)]
enum DaemonEvent {
    Connect(DaemonClient),
    Disconnect(SocketAddr),
    Packet(SocketAddr, Packet),
}

fn run_client_read_thread(mut stream: TcpStream, to_handle: mpsc::Sender<DaemonEvent>) {
    let addr = stream.peer_addr().unwrap();
    loop {
        match Packet::read(&mut stream) {
            Ok(p) => to_handle.send(DaemonEvent::Packet(addr, p)).unwrap(),
            Err(_) => {
                to_handle.send(DaemonEvent::Disconnect(addr)).unwrap();
                return;
            }
        }
    }
}

fn try_listener_accept_client(
    listener: &TcpListener,
    client_sender: &mpsc::Sender<DaemonEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (stream, _) = listener.accept()?;
    let read_stream = stream.try_clone()?;
    let read_sender = client_sender.clone();
    client_sender.send(DaemonEvent::Connect(DaemonClient {
        stream,
        channels: SubscriptionSet::new(),
    }))?;
    thread::spawn(|| run_client_read_thread(read_stream, read_sender));
    Ok(())
}

fn run_listener_thread(port: u16, client_sender: mpsc::Sender<DaemonEvent>) {
    let listener = TcpListener::bind(SocketAddr::new(LOOPBACK_ADDRESS, port)).unwrap();
    println!("lostreams daemon running on port {}", port);
    loop {
        let _ = try_listener_accept_client(&listener, &client_sender);
    }
}

pub fn run_daemon(port: u16) {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || run_listener_thread(port, sender));
    let mut clients = HashMap::new();
    loop {
        let message = receiver.recv().unwrap();
        match message {
            DaemonEvent::Connect(client) => {
                clients.insert(client.stream.peer_addr().unwrap(), client);
            }
            DaemonEvent::Disconnect(addr) => {
                clients.remove(&addr);
            }
            DaemonEvent::Packet(src_addr, packet) => match &packet {
                Packet::Message {
                    channel,
                    content: _,
                } => {
                    for (addr, client) in &mut clients {
                        if *addr != src_addr && client.channels.is_subscribed(channel) {
                            let _ = packet.write(&mut client.stream);
                        }
                    }
                }
                Packet::Subscribe { channel } => {
                    clients
                        .get_mut(&src_addr)
                        .unwrap()
                        .channels
                        .subscribe(channel.clone());
                }
                Packet::SubscribeAll => {
                    clients.get_mut(&src_addr).unwrap().channels.subscribe_all();
                }
                Packet::Unsubscribe { channel } => {
                    clients
                        .get_mut(&src_addr)
                        .unwrap()
                        .channels
                        .unsubscribe(channel.clone());
                }
                Packet::UnsubscribeAll => {
                    clients
                        .get_mut(&src_addr)
                        .unwrap()
                        .channels
                        .unsubscribe_all();
                }
            },
        }
    }
}
