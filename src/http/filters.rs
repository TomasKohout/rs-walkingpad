use crate::controller::Pad;
use warp::{hyper::StatusCode, Filter, Rejection, Reply};

use crate::http::handlers;

use std::{collections::HashMap, convert::Infallible, error::Error};

/// The 4 TODOs filters combined.
pub fn walkingpad<T: btleplug::api::Peripheral>(
    pad: Pad<T>,
) -> impl Filter<Extract = impl warp::Reply, Error = Infallible> + Clone {
    start_belt(pad.clone())
        .or(stop_belt(pad.clone()))
        .or(change_speed(pad.clone()))
        .recover(handle_rejection)
}

/// POST /!start_belt
pub fn start_belt<T: btleplug::api::Peripheral>(
    pad: Pad<T>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("!start_belt")
        .and(warp::post())
        .and(with_pad(pad))
        .and_then(handlers::start_belt)
}

/// POST /!stop_belt
pub fn stop_belt<T: btleplug::api::Peripheral>(
    pad: Pad<T>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("!stop_belt")
        .and(warp::post())
        .and(with_pad(pad))
        .and_then(handlers::stop_belt)
}

pub fn change_speed<T: btleplug::api::Peripheral>(
    pad: Pad<T>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("!change_speed")
        .and(warp::query::<HashMap<String, String>>())
        .and(with_pad(pad))
        .and_then(handlers::change_speed)
}

// pub fn todos_list(
//     pad: Pad<_>,
// ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
//     warp::path!("todos")
//         .and(warp::get())
//         .and(warp::query::<ListOptions>())
//         .and(with_db(pad))
//         .and_then(handlers::list_todos)
// }

// /// POST /todos with JSON body
// pub fn todos_create(
//     pad: Pad<_>,
// ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
//     warp::path!("todos")
//         .and(warp::post())
//         .and(content_length())
//         .and(with_db(pad))
//         .and_then(handlers::create_todo)
// }

// /// PUT /todos/:id with JSON body
// pub fn todos_update(
//     pad: Pad<_>,
// ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
//     warp::path!("todos" / u64)
//         .and(warp::put())
//         .and(content_length())
//         .and(with_db(pad))
//         .and_then(handlers::update_todo)
// }

// /// DELETE /todos/:id
// pub fn todos_delete(
//     pad: Pad<_>,
// ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
//     // We'll make one of our endpoints admin-only to show how authentication filters are used
//     let admin_only = warp::header::exact("authorization", "Bearer admin");

//     warp::path!("todos" / u64)
//         // It is important to put the auth check _after_ the path filters.
//         // If we put the auth check before, the request `PUT /todos/invalid-string`
//         // would try this filter and reject because the authorization header doesn't match,
//         // rather because the param is wrong for that other path.
//         .and(admin_only)
//         .and(warp::delete())
//         .and(with_db(pad))
//         .and_then(handlers::delete_todo)
// }

fn with_pad<T: btleplug::api::Peripheral>(
    pad: Pad<T>,
) -> impl Filter<Extract = (Pad<T>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || pad.clone())
}

// fn content_length() -> impl Filter<Extract = (,), Error = warp::Rejection> + Clone {
//     // When accepting a body, we want a JSON body
//     // (and to reject huge payloads)...
//     warp::body::content_length_limit(1024 * 16)
// }
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if let Some(e) = err.find::<crate::http::handlers::Error>() {
        code = StatusCode::BAD_REQUEST;
        message = &e.reason;
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        // We can handle a specific error, here METHOD_NOT_ALLOWED,
        // and render it however we want
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "METHOD_NOT_ALLOWED";
    } else {
        // We should have expected this... Just log and say its a 500
        eprintln!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    }

    let json = warp::reply::json(&crate::http::handlers::Error {
        reason: message.to_string(),
    });

    Ok(warp::reply::with_status(json, code))
}
