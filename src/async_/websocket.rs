use crate::async_::peer::Peer;
use crate::types::{Error, TRANSPORT_WEB_SOCKET, TransportType};
use async_trait::async_trait;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use tungstenite::{Bytes, Message, Utf8Bytes};

#[derive(Debug, Clone)]
pub struct WebSocketPeer {
    reader: Arc<Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    writer: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    binary: bool,
}

#[async_trait]
impl Peer for WebSocketPeer {
    fn kind(&self) -> TransportType {
        TRANSPORT_WEB_SOCKET
    }

    async fn read(&self) -> Result<Vec<u8>, Error> {
        let mut reader = self.reader.clone().lock_owned().await;
        let out = reader.next().await.unwrap().unwrap();
        Ok(out.into_data().to_vec())
    }

    async fn write(&self, data: Vec<u8>) -> Result<(), Error> {
        let mut writer = self.writer.clone().lock_owned().await;
        if self.binary {
            _ = writer
                .send(Message::Binary(Bytes::copy_from_slice(&data)))
                .await
                .map_err(|e| Error::new(format!("write error: {e}")))?;
            Ok(())
        } else {
            let as_string = String::from_utf8(data).map_err(|e| Error::new(format!("Not valid UTF-8: {e}")))?;
            _ = writer
                .send(Message::Text(Utf8Bytes::from(as_string)))
                .await
                .map_err(|e| Error::new(format!("write error: {e}")))?;
            Ok(())
        }
    }
}

#[allow(clippy::new_ret_no_self)]
impl WebSocketPeer {
    pub fn new(
        reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        writer: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        binary: bool,
    ) -> Box<dyn Peer> {
        Box::new(WebSocketPeer {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
            binary,
        })
    }
}
