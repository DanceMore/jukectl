mod index;
mod queue;
mod song;
mod tags;

pub fn all_routes() -> Vec<rocket::Route> {
    // Combine routes from all modules
    let mut routes = Vec::new();
    routes.extend(index::routes());
    routes.extend(queue::routes());
    routes.extend(song::routes());
    routes.extend(tags::routes());
    routes
}