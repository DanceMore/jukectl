#[macro_use]
extern crate rocket;

// local imports
mod app_state;
use app_state::AppState;
mod routes;
mod scheduler;
use scheduler::start_scheduler;

#[launch]
async fn rocket() -> _ {
    // Initialize is now async
    let app_state = app_state::initialize().await;

    rocket::build()
        .manage(app_state)
        .mount("/", routes::all_routes())
        .attach(rocket::fairing::AdHoc::on_liftoff(
            "Initialize Queue and Scheduler",
            |rocket| {
                Box::pin(async move {
                    let state = rocket.state::<AppState>().unwrap();
                    app_state::initialize_queue(state).await;
                    start_scheduler(state.clone()).await;
                })
            },
        ))
}
