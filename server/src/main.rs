#[macro_use]
extern crate rocket;

// local imports
mod app_state;
use app_state::AppState;
mod routes;
mod scheduler;
use scheduler::start_scheduler;

#[launch]
fn rocket() -> _ {
    // Initialize the app state
    let app_state = app_state::initialize();

    // Build the rocket instance with routes and scheduler
    rocket::build()
        .manage(app_state.clone())
        .mount("/", routes::all_routes())
        .attach(rocket::fairing::AdHoc::on_liftoff(
            "Initialize Queue and Scheduler",
            |rocket| {
                Box::pin(async move {
                    let state = rocket.state::<AppState>().unwrap();
                    app_state::initialize_queue(state).await;
                    start_scheduler(app_state).await;
                })
            },
        ))
}
