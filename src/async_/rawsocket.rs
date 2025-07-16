use crate::async_::peer::Peer;
use crate::types::{Error, SerializerSpec, TRANSPORT_RAW_SOCKET, TransportType};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use url::Url;
use wampproto::transports::rawsocket::{
    DEFAULT_MAX_MSG_SIZE, Handshake, Message as RSMessage, MessageHeader, receive_handshake, receive_message_header,
    send_handshake, send_message_header,
};

#[derive(Debug, Clone)]
pub struct RawSocketPeer {
    reader: Arc<Mutex<ReadHalf<TcpStream>>>,
    writer: Arc<Mutex<WriteHalf<TcpStream>>>,
}

#[async_trait]
impl Peer for RawSocketPeer {
    fn kind(&self) -> TransportType {
        TRANSPORT_RAW_SOCKET
    }

    async fn read(&self) -> Result<Vec<u8>, Error> {
        let mut reader = self.reader.lock().await;

        let mut buf = [0u8; 4];
        reader
            .read(&mut buf)
            .await
            .map_err(|e| Error::new(format!("failed to read handshake response: {e}")))?;

        let header =
            receive_message_header(&buf).map_err(|e| Error::new(format!("failed to read handshake response: {e}")))?;

        let mut buf = vec![0u8; header.length()];
        reader
            .read(&mut buf)
            .await
            .map_err(|e| Error::new(format!("failed to read header response: {e}")))?;

        Ok(buf)
    }

    async fn write(&self, data: Vec<u8>) -> Result<(), Error> {
        let header = MessageHeader::new(RSMessage::Wamp, data.len());
        let header_raw = send_message_header(&header);

        let mut writer = self.writer.lock().await;
        writer
            .write_all(&header_raw)
            .await
            .map_err(|e| Error::new(format!("failed to send header: {e}")))?;

        writer
            .write_all(&data)
            .await
            .map_err(|e| Error::new(format!("failed to send payload: {e}")))?;

        Ok(())
    }
}

#[allow(clippy::new_ret_no_self)]
impl RawSocketPeer {
    pub fn new(reader: ReadHalf<TcpStream>, writer: WriteHalf<TcpStream>) -> Box<dyn Peer> {
        Box::new(RawSocketPeer {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        })
    }
}

pub async fn connect_rawsocket(uri: &str, serializer: Box<dyn SerializerSpec>) -> Result<Box<dyn Peer>, Error> {
    let parsed = Url::parse(uri).map_err(|e| Error::new(format!("invalid uri: {e}")))?;
    let host = parsed.host_str().unwrap();
    let port = parsed.port_or_known_default().unwrap();

    let addr = format!("{host}:{port}");
    let mut stream = TcpStream::connect(addr)
        .await
        .map_err(|e| Error::new(format!("connect error: {e}")))?;

    let handshake = Handshake::new(serializer.serializer_id(), DEFAULT_MAX_MSG_SIZE);

    let handshake_raw =
        send_handshake(&handshake).map_err(|e| Error::new(format!("failed to serialize handshake: {e}")))?;

    stream
        .write_all(&handshake_raw)
        .await
        .map_err(|e| Error::new(format!("failed to send handshake: {e}")))?;

    let mut buf = [0u8; 4];
    stream
        .read(&mut buf)
        .await
        .map_err(|e| Error::new(format!("failed to read handshake response: {e}")))?;

    _ = receive_handshake(&buf).map_err(|e| Error::new(format!("failed to parse handshake response: {e}")))?;

    let (reader, writer) = tokio::io::split(stream);
    Ok(RawSocketPeer::new(reader, writer))
}
