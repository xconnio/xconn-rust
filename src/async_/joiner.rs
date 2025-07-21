use crate::async_::peer::Peer;
use crate::async_::rawsocket::connect_rawsocket;
use crate::async_::websocket::WebSocketPeer;
use crate::common::types::{Error, JSONSerializerSpec, SerializerSpec, SessionDetails};
use futures_util::{StreamExt, TryFutureExt};
use tokio_tungstenite::connect_async_with_config;
use tungstenite::ClientRequestBuilder;
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

    pub async fn join(&self, uri: &str, realm: &str) -> Result<(Box<dyn Peer>, SessionDetails), Error> {
        let uri = uri.parse().unwrap();
        let request = ClientRequestBuilder::new(uri).with_sub_protocol(self.serializer.subprotocol());
        let config = Some(WebSocketConfig::default());

        let (ws, _) = connect_async_with_config(request, config, false)
            .await
            .map_err(|e| Error::new(format!("failed to connect: {e}")))?;
        let (writer, reader) = ws.split();
        let peer = WebSocketPeer::new(reader, writer, self.serializer.is_binary());
        let auth = self.authenticator.clone();
        join(peer, realm, self.serializer.serializer(), auth).await
    }
}

pub async fn join(
    peer: Box<dyn Peer>,
    realm: &str,
    serializer: Box<dyn Serializer>,
    authenticator: Box<dyn ClientAuthenticator>,
) -> Result<(Box<dyn Peer>, SessionDetails), Error> {
    let mut proto = joiner::Joiner::new(realm, serializer.clone(), authenticator);

    let hello_raw = proto
        .send_hello()
        .map_err(|e| Error::new(format!("failed to send hello: {e}")))?;

    peer.write(hello_raw).await?;

    loop {
        let reply = peer
            .read()
            .await
            .map_err(|e| Error::new(format!("failed to read: {e}")))?;

        match proto.receive(reply) {
            Ok(Some(to_send)) => peer.write(to_send).await?,
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

pub struct RawSocketJoiner {
    serializer: Box<dyn SerializerSpec>,
    authenticator: Box<dyn ClientAuthenticator>,
}

impl Default for RawSocketJoiner {
    fn default() -> Self {
        Self::new(
            Box::new(JSONSerializerSpec {}),
            Box::new(AnonymousAuthenticator::default()),
        )
    }
}

impl RawSocketJoiner {
    pub fn new(serializer: Box<dyn SerializerSpec>, authenticator: Box<dyn ClientAuthenticator>) -> Self {
        Self {
            serializer,
            authenticator,
        }
    }

    pub async fn join(&self, uri: &str, realm: &str) -> Result<(Box<dyn Peer>, SessionDetails), Error> {
        let peer = connect_rawsocket(uri, self.serializer.clone())
            .map_err(|e| Error::new(format!("failed to connect: {e}")))
            .await?;

        join(peer, realm, self.serializer.serializer(), self.authenticator.clone()).await
    }
}
