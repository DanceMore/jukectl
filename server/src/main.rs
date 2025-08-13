#[macro_use]
extern crate rocket;

// local imports
mod app_state;
use app_state::AppState;
mod routes;
mod scheduler;
use scheduler::start_scheduler;

mod nolog;
use crate::nolog::MyLoggingFairing;

#[launch]
fn rocket() -> _ {
    let app_state = app_state::initialize();

    rocket::build()
        .manage(app_state)
        .mount("/", routes::all_routes())
        .attach(MyLoggingFairing)
        .attach(rocket::fairing::AdHoc::on_liftoff(
            "Initialize Queue and Scheduler",
            |rocket| {
                Box::pin(async move {
                    let state = rocket.state::<AppState>().unwrap();
                    app_state::initialize_queue(state).await;
                    start_scheduler(state.clone()).await; // Get it from rocket's state
                })
            },
        ))
}
