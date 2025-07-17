use xconn::sync::client::connect_anonymous;
use xconn::sync::types::{CallRequest, Event, Invocation, PublishRequest, RegisterRequest, SubscribeRequest, Yield};

fn main() {
    let session = connect_anonymous("ws://localhost:8080/ws", "realm1").unwrap_or_else(|e| panic!("{e}"));

    fn registration_handler(inv: Invocation) -> Yield {
        Yield::new(inv.args, inv.kwargs)
    }

    let register_request = RegisterRequest::new("io.xconn.echo", registration_handler);
    match session.register(register_request) {
        Ok(response) => println!("{response:?}"),
        Err(e) => println!("{e}"),
    }

    let call_request = CallRequest::new("io.xconn.echo").arg(1).kwarg("name", "John");

    match session.call(call_request) {
        Ok(response) => println!("{response:?}"),
        Err(e) => println!("{e}"),
    }

    fn event_handler(event: Event) {
        println!("received event {event:?}")
    }

    let subscribe_request = SubscribeRequest::new("io.xconn.event", event_handler);
    match session.subscribe(subscribe_request) {
        Ok(response) => println!("{response:?}"),
        Err(e) => println!("{e}"),
    }

    let publish_request = PublishRequest::new("io.xconn.event")
        .arg("hey there!")
        .option("acknowledge", true);

    match session.publish(publish_request) {
        Ok(response) => println!("{response:?}"),
        Err(e) => println!("{e}"),
    }

    session.wait_disconnect();
}
