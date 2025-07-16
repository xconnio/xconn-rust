use xconn::async_::client::connect_anonymous;
use xconn::async_::types::{CallRequest, Event, Invocation, PublishRequest, RegisterRequest, SubscribeRequest, Yield};

#[tokio::main]
async fn main() {
    let session = connect_anonymous("ws://localhost:8080/ws", "realm1")
        .await
        .unwrap_or_else(|e| panic!("{e}"));

    async fn registration_handler(inv: Invocation) -> Yield {
        Yield {
            args: inv.args,
            kwargs: inv.kwargs,
        }
    }

    let register_request = RegisterRequest::new("com.genki.echo", registration_handler);
    match session.register(register_request).await {
        Ok(response) => println!("{response:?}"),
        Err(e) => println!("{e}"),
    }

    let call_request = CallRequest::new("com.genki.echo")
        .with_arg(1)
        .with_kwarg("name", "Robot");

    let response = session.call(call_request).await.unwrap();
    println!("args={:?}, kwargs={:?}", response.args, response.kwargs);

    async fn event_handler(event: Event) {
        println!("received event {event:?}")
    }

    let subscribe_request = SubscribeRequest::new("com.genki.event", event_handler);
    match session.subscribe(subscribe_request).await {
        Ok(response) => println!("{response:?}"),
        Err(e) => println!("{e}"),
    }

    let publish_request = PublishRequest::new("com.genki.event")
        .with_arg("hey there!")
        .with_option("acknowledge", true);

    match session.publish(publish_request).await {
        Ok(response) => println!("{response:?}"),
        Err(e) => println!("{e}"),
    }

    session.wait_disconnect().await;
}
