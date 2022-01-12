use crate::Pad;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt;
use warp::http::StatusCode;

use serde::Serialize;
use warp::{reject, Rejection};

#[derive(Debug, Serialize)]
pub struct Error {
    pub reason: String,
}

impl warp::reject::Reject for Error {}

pub async fn start_belt<T: btleplug::api::Peripheral>(
    pad: Pad<T>,
) -> Result<impl warp::Reply, Infallible> {
    pad.start_belt().await;
    Ok(warp::reply::json(&"Belt Started!".to_string()))
}

pub async fn stop_belt<T: btleplug::api::Peripheral>(
    pad: Pad<T>,
) -> Result<impl warp::Reply, Infallible> {
    pad.stop_belt().await;
    Ok(warp::reply::json(&"Belt Stopped!".to_string()))
}

pub async fn change_speed<T: btleplug::api::Peripheral>(
    query: HashMap<String, String>,
    pad: Pad<T>,
) -> Result<impl warp::Reply, Rejection> {
    let speed = match query.get("speed") {
        Some(spd) => spd.parse(),
        None => Ok(0),
    };

    match speed {
        Ok(speed) => {
            if speed > 60 || speed < 0 {
                Err(reject::custom(Error {
                    reason: format!("Speed not allowed! {}", speed),
                }))
            } else {
                match pad.change_speed(speed as u8).await {
                    Ok(_) => Ok(warp::reply::json(
                        &format!("Speed changed to {}", speed).to_string(),
                    )),
                    Err(err) => Err(reject::custom(Error {
                        reason: format!("There was some internal error! {}", err.to_string()),
                    })),
                }
            }
        }
        Err(_) => Err(reject::custom(Error {
            reason: format!("Speed not provided!"),
        })),
    }
}

// pub async fn create_todo(create: Todo, db: Db) -> Result<impl warp::Reply, Infallible> {
//     log::debug!("create_todo: {:?}", create);

//     let mut vec = db.lock().await;

//     for todo in vec.iter() {
//         if todo.id == create.id {
//             log::debug!("    -> id already exists: {}", create.id);
//             // Todo with id already exists, return `400 BadRequest`.
//             return Ok(StatusCode::BAD_REQUEST);
//         }
//     }

//     // No existing Todo with id, so insert and return `201 Created`.
//     vec.push(create);

//     Ok(StatusCode::CREATED)
// }

// pub async fn update_todo(
//     id: u64,
//     update: Todo,
//     db: Db,
// ) -> Result<impl warp::Reply, Infallible> {
//     log::debug!("update_todo: id={}, todo={:?}", id, update);
//     let mut vec = db.lock().await;

//     // Look for the specified Todo...
//     for todo in vec.iter_mut() {
//         if todo.id == id {
//             *todo = update;
//             return Ok(StatusCode::OK);
//         }
//     }

//     log::debug!("    -> todo id not found!");

//     // If the for loop didn't return OK, then the ID doesn't exist...
//     Ok(StatusCode::NOT_FOUND)
// }

// pub async fn delete_todo(id: u64, db: Db) -> Result<impl warp::Reply, Infallible> {
//     log::debug!("delete_todo: id={}", id);

//     let mut vec = db.lock().await;

//     let len = vec.len();
//     vec.retain(|todo| {
//         // Retain all Todos that aren't this id...
//         // In other words, remove all that *are* this id...
//         todo.id != id
//     });

//     // If the vec is smaller, we found and deleted a Todo!
//     let deleted = vec.len() != len;

//     if deleted {
//         // respond with a `204 No Content`, which means successful,
//         // yet no body expected...
//         Ok(StatusCode::NO_CONTENT)
//     } else {
//         log::debug!("    -> todo id not found!");
//         Ok(StatusCode::NOT_FOUND)
//     }
// }
