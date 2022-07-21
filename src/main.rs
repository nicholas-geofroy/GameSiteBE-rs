use rocket::serde::{Deserialize, json::Json};
mod user_manager;
mod models;
use crate::models::user::User;

#[macro_use] extern crate rocket;

#[get("/")]
fn index() -> &'static str {
    "Hello, world"
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct NewUserRequest<'a> {
    name: &'a str
}

#[post("/register", data="<req>")]
fn register(req: Json<NewUserRequest<'_>>) -> Json<User> {
    Json(user_manager::create_user(req.into_inner()))
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, register])
}
