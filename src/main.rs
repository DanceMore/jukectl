use tide::{Request, Response};
use tide::http::StatusCode;
use serde::Serialize;
use serde_json::json;
//use std::io::Write;

#[derive(Serialize)]
struct Song {
    file: String,
}

fn queue_to_filenames(song_array: Vec<Song>) -> Vec<String> {
    let mut filename_array = Vec::new();

    for song in song_array {
        filename_array.push(song.file);
    }

    filename_array
}

async fn now_playing(_: Request<()>) -> tide::Result {
    // Simulate fetching a list of songs, replace this with your actual logic
    let song1 = Song {
        file: "song1.mp3".to_string(),
    };
    let song2 = Song {
        file: "song2.mp3".to_string(),
    };
    let song3 = Song {
        file: "song3.mp3".to_string(),
    };
    let song_array = vec![song1, song2, song3];

    let res = queue_to_filenames(song_array);

    let json_response = json!(res).to_string();

    Ok(Response::builder(StatusCode::Ok)
        .body(tide::Body::from_string(json_response))
        .content_type("application/json")
        .build())
}

use std::io::Write;

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
    // Configure tracing to use a subscriber for logging
    //let subscriber = tracing_subscriber::fmt()
    //    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    //    .json()
    //    .finish();

    //tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    //let mut app = tide::Server::new();
    let mut app = tide::new();

    app.at("/").get(now_playing);

    let addr = "127.0.0.1:8080";
    println!("Server listening on {}", addr);
    let server = app.listen(addr);

    // Spawn a detached asynchronous task to run the scheduler_mainbody function
    tokio::spawn(scheduler_mainbody());

    // Wait for the Tide server to finish
    server.await?;
    Ok(())
}
