mod daemon;

use std::{
    fs,
    io::{Read, Write},
    path::Path,
    sync::mpsc,
};

use clap::{Parser, Subcommand};
use modstreams_core::{ModstreamsClient, Packet};
use notify::Watcher;

#[derive(Subcommand)]
enum Command {
    /// Run the lostreams daemon
    Run,
    /// Log messages from a set of channels
    Debug {
        /// The channels to log
        channels: Vec<String>,
    },
    /// Read all messages from a set of channels and write them to standard out
    Out {
        /// The channels to read from
        channels: Vec<String>,

        /// Automatically flush standard out after each message
        #[arg(short = 'f', long = "flush", default_value_t = false)]
        flush: bool,
    },
    /// Read all messages from standard in and write them to a set of channels
    In {
        /// The channels to write to
        channels: Vec<String>,

        // The size of the input buffer
        #[arg(short = 'b', long = "buffer", default_value_t = 4096)]
        buffer_size: usize,
    },
    /// Send a value to a channel
    Send {
        /// The channel to send the value to
        channel: String,
        /// The value to send
        value: String,
    },
    /// Copy messages from set of channels to another
    Copy {
        /// Input streams
        #[arg(short = 'i', long = "input")]
        src_channels: Vec<String>,

        /// Output streams
        #[arg(short = 'o', long = "output")]
        dest_channels: Vec<String>,
    },
    /// Send the contents of a file to a channel every time it changes
    Watch { channel: String, file_path: String },
}

#[derive(Parser)]
#[command(version = "1.0")]
struct Args {
    #[arg(short = 'p', long = "port", default_value_t = 7460)]
    port: u16,
    #[command(subcommand)]
    command: Command,
}

fn run_debug(port: u16, channels: Vec<String>) {
    let mut client = ModstreamsClient::new(port);
    if channels.len() == 0 {
        client.subscribe_all().unwrap();
    } else {
        for channel in &channels {
            client.subscribe(channel).unwrap();
        }
    }
    loop {
        if let Packet::Message { channel, content } = client.read().unwrap() {
            let len = content.len();
            if let Ok(s) = String::from_utf8(content) {
                println!("[{}] '{}'", channel, s);
            } else {
                println!("[{}] <{} bytes>", channel, len);
            }
        }
    }
}

fn run_out(port: u16, channels: Vec<String>, flush: bool) {
    let mut client = ModstreamsClient::new(port);
    if channels.len() == 0 {
        client.subscribe_all().unwrap();
    } else {
        for channel in &channels {
            client.subscribe(channel).unwrap();
        }
    }
    let mut stdout = std::io::stdout();
    loop {
        if let Packet::Message {
            channel: _,
            content,
        } = client.read().unwrap()
        {
            stdout.write_all(&content).unwrap();
            if flush {
                stdout.flush().unwrap();
            }
        }
    }
}

fn run_in(port: u16, channels: Vec<String>, buffer_size: usize) {
    let mut client = ModstreamsClient::new(port);
    let mut stdin = std::io::stdin();
    let mut buffer = vec![0u8; buffer_size];
    loop {
        let bytes_read = stdin.read(&mut buffer).unwrap();
        if bytes_read == 0 {
            break;
        }
        for channel in &channels {
            client.send(channel, &buffer[..bytes_read]).unwrap();
        }
    }
}

fn run_copy(port: u16, src_channels: Vec<String>, dest_channels: Vec<String>) {
    let mut client = ModstreamsClient::new(port);
    for src_channel in &src_channels {
        client.subscribe(src_channel).unwrap();
    }
    loop {
        if let Packet::Message { channel, content } = client.read().unwrap() {
            for dest_channel in &dest_channels {
                if *dest_channel != channel {
                    client.send(dest_channel, &content).unwrap();
                }
            }
        }
    }
}

fn run_watch(port: u16, channel: String, file_path: String) {
    let mut client = ModstreamsClient::new(port);
    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(tx).unwrap();
    watcher
        .watch(Path::new(&file_path), notify::RecursiveMode::Recursive)
        .unwrap();
    client
        .send(&channel, &fs::read(&file_path).unwrap())
        .unwrap();
    for res in rx {
        let ev = res.unwrap();
        if let notify::EventKind::Modify(notify::event::ModifyKind::Data(_)) = ev.kind {
            client
                .send(&channel, &fs::read(&file_path).unwrap())
                .unwrap();
        }
    }
}

fn main() {
    let args = Args::parse();
    match args.command {
        Command::Run => daemon::run_daemon(args.port),
        Command::Send { channel, value } => {
            let mut client = ModstreamsClient::new(args.port);
            client.send(&channel, value.as_bytes()).unwrap()
        }
        Command::Debug { channels } => run_debug(args.port, channels),
        Command::Out { channels, flush } => run_out(args.port, channels, flush),
        Command::In {
            channels,
            buffer_size,
        } => run_in(args.port, channels, buffer_size),
        Command::Copy {
            src_channels,
            dest_channels,
        } => run_copy(args.port, src_channels, dest_channels),
        Command::Watch { channel, file_path } => run_watch(args.port, channel, file_path),
    }
}
