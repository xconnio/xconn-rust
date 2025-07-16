# WAMP for Rust
This library implements the WAMP protocol. It supports both sync and async variant. The API is identical between the two
implementations.

```rust
use xconn::async_::client::connect_anonymous;

#[tokio::main]
async fn main() {
    // connecting without any authentication
    let session = connect_anonymous("ws://localhost:8080/ws", "realm1")
        .await
        .unwrap_or_else(|e| panic!("{e}"));

    // build a call request
    let call_request = CallRequest::new("io.xconn.echo")
        .with_arg(1)
        .with_kwarg("name", "John");

    // pass the request and get the response.
    let response = session.call(call_request).await.unwrap();
    println!("args={:?}, kwargs={:?}", response.args, response.kwargs);
}
```
