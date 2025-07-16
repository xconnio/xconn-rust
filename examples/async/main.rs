use xconn::async_::client::connect_anonymous;
use xconn::async_::session::Session;
use xconn::async_::types::{CallRequest, Event, Invocation, PublishRequest, RegisterRequest, SubscribeRequest, Yield};

#[tokio::main]
async fn main() {
    match connect_anonymous("ws://localhost:8080/ws", "realm1").await {
        Ok(session) => _ = do_actions(session).await,
        Err(e) => println!("{e}"),
    }
}

async fn do_actions(session: Session) {
    async fn registration_handler(inv: Invocation) -> Yield {
        Yield {
            args: inv.args,
            kwargs: inv.kwargs,
        }
    }

    let register_request = RegisterRequest::new("io.xconn.echo", registration_handler);
    match session.register(register_request).await {
        Ok(response) => println!("{response:?}"),
        Err(e) => println!("{e}"),
    }

    let call_request = CallRequest::new("io.xconn.echo")
        .with_arg(1)
        .with_kwarg("name", "John");

    match session.call(call_request).await {
        Ok(response) => println!("{response:?}"),
        Err(e) => println!("{e}"),
    }

    async fn event_handler(event: Event) {
        println!("received event {event:?}")
    }

    let subscribe_request = SubscribeRequest::new("io.xconn.event", event_handler);
    match session.subscribe(subscribe_request).await {
        Ok(response) => println!("{response:?}"),
        Err(e) => println!("{e}"),
    }

    let publish_request = PublishRequest::new("io.xconn.event")
        .with_arg("hey there!")
        .with_option("acknowledge", true);

    match session.publish(publish_request).await {
        Ok(response) => println!("{response:?}"),
        Err(e) => println!("{e}"),
    }

    session.wait_disconnect().await
}
