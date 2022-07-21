use uuid::Uuid;

use crate::models::user::User;
use crate::NewUserRequest;

pub fn create_user(req: NewUserRequest) -> User {
    User { name: req.name.to_string(), id: Uuid::new_v4() }
}
