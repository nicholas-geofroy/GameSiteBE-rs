use uuid::Uuid;
use rocket::serde::{Serialize};

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct User {
   pub name: String,
   pub id: Uuid
}


