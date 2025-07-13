use crate::sync::peer::peer::{Peer, TRANSPORT_WEB_SOCKET, TransportType};
use crate::types::Error;
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use std::fmt::Debug;
use std::net::TcpStream;
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Bytes, Message, Utf8Bytes, WebSocket};

fn extract_raw_fd(stream: &MaybeTlsStream<TcpStream>) -> Option<RawFd> {
    match stream {
        MaybeTlsStream::Plain(tcp_stream) => Some(tcp_stream.as_raw_fd()),
        MaybeTlsStream::NativeTls(tls_stream) => Some(tls_stream.get_ref().as_raw_fd()),
        _ => None,
    }
}

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
        match reader.recv() {
            Ok(msg) => Ok(msg.into_data().to_vec()),
            Err(e) => Err(Error::new(format!("read error: {e}"))),
        }
    }

    fn write(&self, data: Vec<u8>) -> Result<(), Error> {
        if self.binary {
            match self.writer.send(Message::Binary(Bytes::copy_from_slice(&data))) {
                Ok(()) => Ok(()),
                Err(e) => Err(Error::new(format!("write error: {e}"))),
            }
        } else {
            match String::from_utf8(data) {
                Ok(data) => match self.writer.send(Message::Text(Utf8Bytes::from(data))) {
                    Ok(()) => Ok(()),
                    Err(e) => Err(Error::new(format!("write error: {e}"))),
                },
                Err(e) => Err(Error::new(format!("Not valid UTF-8: {e}"))),
            }
        }
    }
}

#[allow(clippy::new_ret_no_self)]
impl WebSocketPeer {
    pub fn new(conn: WebSocket<MaybeTlsStream<TcpStream>>, binary: bool) -> Box<dyn Peer> {
        let conn = Arc::new(Mutex::new(conn));
        let ws_writer = Arc::clone(&conn);
        let ws_reader = Arc::clone(&conn);

        let (front_writer, background_reader): (mpsc::Sender<Message>, mpsc::Receiver<Message>) = mpsc::channel();
        let (background_writer, front_reader): (mpsc::Sender<Message>, mpsc::Receiver<Message>) = mpsc::channel();

        thread::spawn(move || unsafe {
            let raw_fd: BorrowedFd = {
                let sock = ws_reader.lock().unwrap();
                let raw_fd = extract_raw_fd(sock.get_ref());
                BorrowedFd::borrow_raw(raw_fd.unwrap())
            };

            loop {
                let mut fds = [PollFd::new(raw_fd, PollFlags::POLLIN)];
                match poll(&mut fds, PollTimeout::MAX) {
                    Ok(0) => {
                        // Not data yet.
                        continue;
                    }
                    Ok(_) => {
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

                    Err(e) => {
                        eprintln!("[Reader] Error: {e}");
                        break;
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

        Box::new(Self {
            kind: TRANSPORT_WEB_SOCKET,
            reader: Arc::new(Mutex::new(front_reader)),
            writer: Arc::new(front_writer),
            binary,
        })
    }
}
