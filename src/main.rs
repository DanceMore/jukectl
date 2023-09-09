use serde_json::json;
use std::env;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tide::http::StatusCode;
use tide::{Request, Response};

mod mpd_conn;
use crate::mpd_conn::MpdConn;

mod queue;
use crate::queue::Queue;

mod tags_data;
use crate::tags_data::TagsData;

fn queue_to_filenames(song_array: Vec<mpd::Song>) -> Vec<String> {
    let mut filename_array = Vec::new();

    for song in song_array {
        filename_array.push(song.file);
    }

    filename_array
}

async fn now_playing(_: Request<()>) -> tide::Result {
    let mut conn = MpdConn::new()?;
    let song_array = conn.mpd.queue().unwrap();

    let res = queue_to_filenames(song_array);

    let json_response = json!(res).to_string();

    Ok(tide::Response::builder(tide::StatusCode::Ok)
        .body(tide::Body::from_string(json_response))
        .content_type("application/json")
        .build())
}

async fn get_tags(_: tide::Request<()>, tags_data: Arc<Mutex<TagsData>>) -> tide::Result {
    // Serialize TagsData to a JSON string
    let tags_data_json = serde_json::to_string(&*tags_data)?;

    // Return a response with the JSON string
    Ok(tide::Response::builder(tide::StatusCode::Ok)
        .body(tide::Body::from_string(tags_data_json))
        .content_type("application/json")
        .build())
}

//async fn get_tags(req: Request<()>) -> tide::Result {
//    // Retrieve TagsData from the request state
//    let tags_data = req.state().clone();
//
//    // Lock the Mutex to access TagsData
//    let tags_data = tags_data.lock().await;
//
//    // Serialize TagsData to a JSON string
//    let tags_data_json = serde_json::to_string(&*tags_data)?;
//
//    // Return a response with the JSON string
//    Ok(tide::Response::builder(200)
//        .body(tide::Body::from_string(tags_data_json))
//        .content_type("application/json")
//        .build())
//}


async fn scheduler_mainbody() {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3));
    loop {
        interval.tick().await;
        print!(".");
        std::io::stdout().flush().expect("Failed to flush stdout");
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::init();

    // Read the BIND_HOST and BIND_PORT environment variables with default values
    let bind_host = env::var("BIND_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let bind_port = env::var("BIND_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{}:{}", bind_host, bind_port);

    // Shareable TagsData with default values
    let default_tags_data = TagsData {
        any: vec!["jukebox".to_string()],
        not: vec!["explicit".to_string()],
    };
    let tags_data = Arc::new(Mutex::new(default_tags_data));

    // Shareable Queue
    let queue = Arc::new(Mutex::new(Queue::new()));

    // start building the app itself
    let mut app = tide::new();

    // routes
    app.at("/").get(now_playing);
    app.at("/tags").get(move |_req| get_tags(_req, Arc::clone(&tags_data)));

    // bind the server to listen
    println!("Server listening on {}", addr);
    let server = app.listen(addr);

    // Spawn a detached asynchronous task to run the scheduler_mainbody function
    tokio::spawn(scheduler_mainbody());

    // Wait for the Tide server to finish
    server.await?;
    Ok(())
}
