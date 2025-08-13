use rocket::{fairing::{Fairing, Info, Kind}, Request, Data, Response};

pub struct MyLoggingFairing;

#[rocket::async_trait]
impl Fairing for MyLoggingFairing {
    fn info(&self) -> Info {
        Info {
            name: "My Logging Fairing",
            kind: Kind::Request | Kind::Response
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _data: &mut Data<'_>) {
        if request.uri().path().starts_with("/tags") {
            // Skip logging for routes starting with "/no-log"
            // You might implement custom logic here to prevent logging
            // For example, by not forwarding the request to a logger
            println!("Skipping logging for: {}", request.uri().path());
            request.local_cache(|| false); // Store a flag to indicate no logging
        } else {
            request.local_cache(|| true); // Store a flag to indicate logging
        }
    }
}
