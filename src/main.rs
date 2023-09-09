#[macro_use]
extern crate rocket;

use rocket::http::Status;
use rocket::tokio::time::{interval, Duration};
use rocket::{Rocket, State};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::{Arc, Mutex};
use std::thread;
//use rocket::response::status;
//use rocket::response::content;
use rocket::serde::json::Json;
use serde_json::json;

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

use std::io::Write;

fn scheduler_mainbody() {
    loop {
        print!(".");
        let _ = std::io::stdout().flush();
        thread::sleep(Duration::from_secs(3));
    }
}

#[get("/")]
fn index() -> Json<Vec<String>> {
    let mut conn = MpdConn::new().unwrap(); // You might want to handle errors differently
    let song_array = conn.mpd.queue().unwrap();

    let res = queue_to_filenames(song_array);

    Json(res)
}

#[get("/tags")]
fn tags(tags_data: &State<Arc<Mutex<TagsData>>>) -> Json<TagsData> {
    let locked_tags_data = tags_data.lock().expect("Failed to lock TagsData");
    Json(locked_tags_data.clone())
}

#[post("/tags", data = "<tags_data>")]
fn update_tags(
    tags_data: Json<TagsData>,
    shared_tags_data: &State<Arc<Mutex<TagsData>>>,
) -> Json<TagsData> {
    let mut locked_data = shared_tags_data.lock().expect("Failed to lock TagsData");
    *locked_data = tags_data.0.clone();
    Json(locked_data.clone())
}

//#[rocket::main]
//async fn main() -> Result<(), rocket::Error> {
//    env_logger::init();
//
//    // Read the BIND_HOST and BIND_PORT environment variables with default values
//    let bind_host = env::var("BIND_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
//    let bind_port = env::var("BIND_PORT").unwrap_or_else(|_| "8080".to_string());
//    let addr = format!("{}:{}", bind_host, bind_port);
//
//
//    // Launch the Rocket app
//    rocket::ignite().mount("/hello", routes![world]);
//
//
//    Ok(())
//}

#[launch]
fn rocket() -> _ {
    // Shareable TagsData with default values
    let default_tags_data = TagsData {
        any: vec!["jukebox".to_string()],
        not: vec!["explicit".to_string()],
    };
    let tags_data = Arc::new(Mutex::new(default_tags_data));

    // Spawn a detached asynchronous task to run the scheduler_mainbody function
    thread::spawn(|| scheduler_mainbody());

    rocket::build()
        .manage(tags_data) // Pass TagsData as a state
        .mount("/", routes![index, tags, update_tags])
}
