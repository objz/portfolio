pub mod sl;
pub mod steam;
pub mod user;

use user::user;

#[derive(Clone)]
pub struct AsciiArt;

impl AsciiArt {
    pub fn get_user() -> String {
        user()
    }
}
