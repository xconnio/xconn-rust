use crate::common::types::{Error, TRANSPORT_WEB_SOCKET, TransportType};
use crate::sync::peer::Peer;
use mio::net::TcpStream as MioTcpStream;
use mio::{Events, Interest, Poll, Token};
use std::fmt::Debug;
use std::net::TcpStream;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use tungstenite::protocol::Role;
use tungstenite::{Bytes, Message, Utf8Bytes, WebSocket};

const CLIENT: Token = Token(0);

#[derive(Debug, Clone)]
pub struct WebSocketPeer {
    kind: TransportType,
    reader: Arc<Mutex<mpsc::Receiver<Message>>>,
    writer: Arc<mpsc::Sender<Message>>,
    binary: bool,
}

impl Peer for WebSocketPeer {
    fn kind(&self) -> TransportType {
        self.kind
    }

    fn read(&self) -> Result<Vec<u8>, Error> {
        let reader = self.reader.lock().unwrap();
        let msg = reader.recv().map_err(|e| Error::new(format!("read error: {e}")))?;
        Ok(msg.into_data().to_vec())
    }

    fn write(&self, data: Vec<u8>) -> Result<(), Error> {
        if self.binary {
            self.writer
                .send(Message::Binary(Bytes::copy_from_slice(&data)))
                .map_err(|e| Error::new(format!("write error: {e}")))?;
            Ok(())
        } else {
            let as_string = String::from_utf8(data).map_err(|e| Error::new(format!("Not valid UTF-8: {e}")))?;
            self.writer
                .send(Message::Text(Utf8Bytes::from(as_string)))
                .map_err(|e| Error::new(format!("write error: {e}")))?;
            Ok(())
        }
    }
}

impl WebSocketPeer {
    pub fn try_new(stream: TcpStream, binary: bool) -> Result<Box<dyn Peer>, Error> {
        let stream_copy = stream
            .try_clone()
            .map_err(|e| Error::new(format!("clone error: {e}")))?;
        let mio_stream = MioTcpStream::from_std(stream_copy);
        let mut mio_stream_b = MioTcpStream::from_std(stream);

        let ws = WebSocket::from_raw_socket(mio_stream, Role::Client, None);
        let ws_conn = Arc::new(Mutex::new(ws));
        let ws_writer = Arc::clone(&ws_conn);
        let ws_reader = Arc::clone(&ws_conn);

        let (front_writer, background_reader): (mpsc::Sender<Message>, mpsc::Receiver<Message>) = mpsc::channel();
        let (background_writer, front_reader): (mpsc::Sender<Message>, mpsc::Receiver<Message>) = mpsc::channel();

        let mut poll = Poll::new().map_err(|e| Error::new(format!("poll error: {e}")))?;
        poll.registry()
            .register(&mut mio_stream_b, CLIENT, Interest::READABLE | Interest::WRITABLE)
            .map_err(|e| Error::new(format!("register error: {e}")))?;
        let mut events = Events::with_capacity(1024);

        thread::spawn(move || {
            loop {
                poll.poll(&mut events, None).unwrap();
                for event in events.iter() {
                    if event.token() == CLIENT && event.is_readable() {
                        let msg_result = {
                            let mut sock = ws_reader.lock().unwrap();
                            sock.read()
                        };

                        match msg_result {
                            Ok(msg) => background_writer.send(msg).unwrap(),
                            Err(e) => {
                                eprintln!("[Reader] Error: {e}");
                                break;
                            }
                        }
                    }
                }
            }
        });

        thread::spawn(move || {
            for msg in background_reader {
                let mut sock = ws_writer.lock().unwrap();
                if let Err(e) = sock.send(msg) {
                    eprintln!("[Writer] Error sending message: {e}");
                    break;
                }
            }
        });

        Ok(Box::new(Self {
            kind: TRANSPORT_WEB_SOCKET,
            reader: Arc::new(Mutex::new(front_reader)),
            writer: Arc::new(front_writer),
            binary,
        }))
    }
}
