use crate::log::log;
use chrono::Utc;
use futures::{StreamExt, TryStreamExt};
use nanoid::nanoid;
use std::{env, fs, path::Path};
use warp::{
    filters::{header::header, multipart::FormData},
    http::StatusCode,
    reject::{self, MissingHeader, Rejection},
    reply, Buf, Filter, Reply,
};

#[derive(Debug)]
struct SystemError {
    msg: String,
}

#[derive(Debug)]
struct Unauthorized;

impl reject::Reject for SystemError {}
impl reject::Reject for Unauthorized {}

pub async fn start() {
    let upload_path = warp::path!("upload")
        .and(warp::post())
        .and(header("ACCESS-KEY"))
        .and(warp::multipart::form().max_length(5_000_000_000))
        .and_then(upload);

    let router = upload_path.recover(handle_rejection);

    let port: u16 = env::var("PORT")
        .expect("Server port undefined")
        .parse()
        .expect("Invalid port number");

    log("info", &format!("Starting server on port {}", port)).expect("Failed to log startup info");

    warp::serve(router).run(([127, 0, 0, 1], port)).await;
}

async fn upload(key: String, form: FormData) -> Result<impl Reply, Rejection> {
    let access_key = env::var("ACCESS_KEY").map_err(|_| {
        reject::custom(SystemError {
            msg: "ACCESS_KEY undefined".to_owned(),
        })
    })?;

    if key != access_key {
        return Err(reject::custom(Unauthorized));
    }

    let mut parts = form.into_stream();
    let mut uploaded_files = Vec::new();

    while let Some(Ok(part)) = parts.next().await {
        let filename = part.filename().unwrap_or("unnamed.txt").to_owned();
        let extension = Path::new(&filename).extension().unwrap();

        let stream_data = part
            .stream()
            .try_fold(Vec::new(), |mut fvec, fbuf| async move {
                fvec.extend_from_slice(fbuf.chunk());
                Ok(fvec)
            })
            .await
            .map_err(|_| {
                reject::custom(SystemError {
                    msg: "Couldn't fold form data".to_owned(),
                })
            })?; // Fold stream data into one vector

        let upload_dir = env::var("UPLOAD_DIR").map_err(|_| {
            reject::custom(SystemError {
                msg: "UPLOAD_DIR undefined".to_owned(),
            })
        })?;
        let upload_dir = Path::new(&upload_dir);

        if !upload_dir.exists() {
            println!("Creating upload directory at {}", upload_dir.display());
            fs::create_dir(upload_dir).map_err(|_| {
                reject::custom(SystemError {
                    msg: "Couldn't create upload directory".to_owned(),
                })
            })?;
        }

        let date = Utc::now().format("%Y/%m/%d").to_string();
        let date_path = Path::new(&date);
        let current_upload_dir = upload_dir.join(date_path.clone());
        fs::create_dir_all(&current_upload_dir).map_err(|_| {
            reject::custom(SystemError {
                msg: "Couldn't create upload path".to_owned(),
            })
        })?;

        let new_filename = format!("{}.{}", nanoid!(), extension.to_str().unwrap());
        let upload_path = current_upload_dir.join(new_filename);

        fs::write(&upload_path, &stream_data).map_err(|_| {
            reject::custom(SystemError {
                msg: "Couldn't write to file".to_owned(),
            })
        })?;

        uploaded_files.push(format!("{}/{}", date_path.display(), filename));
    }

    Ok(uploaded_files.join(" "))
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    println!("{:#?}", err);
    let (message, code) = if err.is_not_found() {
        ("NOT FOUND", StatusCode::NOT_FOUND)
    } else if err.find::<MissingHeader>().is_some()
        || err.find::<reject::PayloadTooLarge>().is_some()
    {
        ("BAD_REQUEST", StatusCode::BAD_REQUEST)
    } else if let Some(e) = err.find::<SystemError>() {
        log("warn", &e.msg).unwrap_or_else(|_| {
            println!("Unable to log error data to log file");
        });
        ("SYS_ERROR", StatusCode::INTERNAL_SERVER_ERROR)
    } else if err.find::<Unauthorized>().is_some() {
        ("Unauthorized", StatusCode::UNAUTHORIZED)
    } else {
        ("INTERNAL_SERVER_ERROR", StatusCode::INTERNAL_SERVER_ERROR)
    };

    Ok(reply::with_status(message, code))
}
