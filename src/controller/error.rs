use derive_more::{Display, Error as DError};

#[derive(Display, Debug, DError)]
pub struct MyError {
    pub details: String,
}
