use xconn::sync::client::connect_anonymous;
use xconn::sync::session::Session;
use xconn::sync::types::{RegisterRequest, SubscribeRequest};
use xconn::types::{CallRequest, Event, Invocation, PublishRequest, Yield};

fn main() {
    match connect_anonymous("ws://localhost:8080/ws", "realm1") {
        Ok(session) => do_actions(session),
        Err(e) => println!("{e}"),
    }
}

fn do_actions(session: Session) {
    fn registration_handler(inv: Invocation) -> Yield {
        Yield {
            args: inv.args,
            kwargs: inv.kwargs,
        }
    }

    let register_request = RegisterRequest::new("io.xconn.echo", registration_handler);
    match session.register(register_request) {
        Ok(response) => println!("{response:?}"),
        Err(e) => println!("{e}"),
    }

    let call_request = CallRequest::new("io.xconn.echo")
        .with_arg(1)
        .with_kwarg("name", "John");

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
        .with_arg("hey there!")
        .with_option("acknowledge", true);

    match session.publish(publish_request) {
        Ok(response) => println!("{response:?}"),
        Err(e) => println!("{e}"),
    }
}
