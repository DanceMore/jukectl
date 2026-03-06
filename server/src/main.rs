#[macro_use]
extern crate rocket;

use jukectl_server::app_state;
use jukectl_server::routes;
use jukectl_server::scheduler;

#[launch]
async fn rocket() -> _ {
    let state = app_state::initialize().await;
    let state_for_scheduler = state.clone();

    rocket::build()
        .manage(state)
        .mount("/", routes::all_routes())
        .attach(rocket::fairing::AdHoc::on_liftoff("Scheduler", |_| {
            Box::pin(async move {
                scheduler::start_scheduler(state_for_scheduler).await;
            })
        }))
}
