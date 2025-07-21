use crate::common::types::{Error, JSONSerializerSpec, SerializerSpec, SessionDetails};
use crate::sync::peer::Peer;
use crate::sync::websocket::WebSocketPeer;
use std::net::{TcpStream, ToSocketAddrs};
use tungstenite::client::IntoClientRequest;
use tungstenite::{ClientHandshake, ClientRequestBuilder};
use url::Url;
use wampproto::authenticators::anonymous::AnonymousAuthenticator;
use wampproto::authenticators::authenticator::ClientAuthenticator;
use wampproto::joiner;
use wampproto::serializers::serializer::Serializer;

pub struct WebSocketJoiner {
    serializer: Box<dyn SerializerSpec>,
    authenticator: Box<dyn ClientAuthenticator>,
}

impl Default for WebSocketJoiner {
    fn default() -> Self {
        Self::new(
            Box::new(JSONSerializerSpec {}),
            Box::new(AnonymousAuthenticator::default()),
        )
    }
}

/// This function opens a tcp stream, and upgrades that to websocket.
/// It then returns the tcp socket itself so that it can be used for doing
/// multithreaded IO.
fn connect_and_upgrade(addr: &str, subprotocol: &str) -> Result<TcpStream, Error> {
    // Parse URI and extract host/port
    let uri = addr
        .parse::<Url>()
        .map_err(|e| Error::new(format!("Invalid URI: {e}")))?;

    let host = uri
        .host_str()
        .ok_or_else(|| Error::new("Missing host in URI".to_string()))?;

    let port = uri
        .port_or_known_default()
        .ok_or_else(|| Error::new("Missing or invalid port".to_string()))?;

    // Connect to the socket
    let socket_addr = (host, port)
        .to_socket_addrs()
        .map_err(|e| Error::new(format!("Failed to resolve address: {e}")))?
        .next()
        .ok_or_else(|| Error::new("Could not resolve any addresses".to_string()))?;

    let stream = TcpStream::connect(socket_addr).map_err(|e| Error::new(format!("Connection failed: {e}")))?;

    // Perform WebSocket handshake
    let request = ClientRequestBuilder::new(uri.as_str().parse().unwrap()).with_sub_protocol(subprotocol);

    let handshake = ClientHandshake::start(
        stream
            .try_clone()
            .map_err(|e| Error::new(format!("Failed to clone stream: {e}")))?,
        request
            .into_client_request()
            .map_err(|e| Error::new(format!("Invalid client request: {e}")))?,
        None,
    )
    .map_err(|e| Error::new(format!("Handshake initialization failed: {e}")))?;

    handshake
        .handshake()
        .map_err(|e| Error::new(format!("Handshake failed: {e}")))?;

    Ok(stream)
}

impl WebSocketJoiner {
    pub fn new(serializer: Box<dyn SerializerSpec>, authenticator: Box<dyn ClientAuthenticator>) -> Self {
        Self {
            serializer,
            authenticator,
        }
    }

    pub fn join(&self, uri: &str, realm: &str) -> Result<(Box<dyn Peer>, SessionDetails), Error> {
        let conn = connect_and_upgrade(uri, self.serializer.subprotocol().as_str())?;
        let peer = WebSocketPeer::try_new(conn, self.serializer.is_binary())?;
        let auth = self.authenticator.clone();
        join(peer, realm, self.serializer.serializer(), auth)
    }
}

pub fn join(
    peer: Box<dyn Peer>,
    realm: &str,
    serializer: Box<dyn Serializer>,
    authenticator: Box<dyn ClientAuthenticator>,
) -> Result<(Box<dyn Peer>, SessionDetails), Error> {
    let mut proto = joiner::Joiner::new(realm, serializer.clone(), authenticator);

    let hello_raw = proto
        .send_hello()
        .map_err(|e| Error::new(format!("failed to send hello: {e}")))?;
    peer.write(hello_raw)?;

    loop {
        if let Ok(reply) = peer.read() {
            match proto.receive(reply) {
                Ok(Some(to_send)) => peer.write(to_send)?,
                Ok(None) => {
                    if let Ok(Some(details)) = proto.session_details() {
                        let details = SessionDetails::new(
                            details.id,
                            details.realm.to_string(),
                            details.authid.to_string(),
                            details.auth_role.to_string(),
                        );

                        return Ok((peer, details));
                    }
                }
                Err(e) => return Err(Error::new(format!("failed to join: {e}"))),
            }
        }
    }
}
