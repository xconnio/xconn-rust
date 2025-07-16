use crate::sync::peer::Peer;
use crate::sync::websocket::WebSocketPeer;
use crate::types::{Error, JSONSerializerSpec, SerializerSpec, SessionDetails};
use http::Uri;
use std::str::FromStr;
use tungstenite::ClientRequestBuilder;
use tungstenite::client::connect_with_config;
use tungstenite::protocol::WebSocketConfig;
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

impl WebSocketJoiner {
    pub fn new(serializer: Box<dyn SerializerSpec>, authenticator: Box<dyn ClientAuthenticator>) -> Self {
        Self {
            serializer,
            authenticator,
        }
    }

    pub fn join(&self, uri: &str, realm: &str) -> Result<(Box<dyn Peer>, SessionDetails), Error> {
        let uri = Uri::from_str(uri).unwrap();
        let request = ClientRequestBuilder::new(uri).with_sub_protocol(self.serializer.subprotocol());
        let config = Some(WebSocketConfig::default());

        let (conn, _) =
            connect_with_config(request, config, 1).map_err(|e| Error::new(format!("failed to connect: {e}")))?;
        let peer = WebSocketPeer::new(conn, self.serializer.is_binary());
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
