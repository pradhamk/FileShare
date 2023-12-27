mod record;

use chrono::Local;
use clap::Parser;
use copypasta::{ClipboardContext, ClipboardProvider};
use dotenv::dotenv;
use futures_util::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use mime;
use notify_rust::Notification;
use record::{create_record, Record};
use reqwest::{multipart, Body, Client};
use std::{
    cmp::min,
    env, fs,
    path::{Path, PathBuf},
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_delimiter = ' ', num_args = 1..)]
    file_path: Vec<String>,

    #[arg(short, long, help = "Upload all files in directory")]
    directory: Option<String>,

    #[arg(short, long)]
    records_path: Option<String>,

    #[arg(short, long, help = "Suppress output", default_value_t = false)]
    quiet: bool,
}

#[tokio::main]
async fn main() {
    dotenv::from_path(env::var("ENV_FILE").unwrap_or(".env".to_owned())).ok();

    let args = Args::parse();

    let mut file_paths: Vec<PathBuf> = args
        .file_path
        .iter()
        .map(|path| Path::new(path).to_owned())
        .collect();

    if let Some(provided_dir) = args.directory.as_deref() {
        let directory = Path::new(provided_dir);

        if directory.exists() {
            let paths = fs::read_dir(directory).unwrap();
            file_paths.extend(
                paths
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
                    .map(|entry| entry.path()),
            );
        } else {
            eprintln!("Provided directory does not exist");
        }
    }

    let mut ctx = ClipboardContext::new().expect("Failed to initialize clipboard context");

    let message = format!("Uploading {} file[s]", file_paths.len());
    if args.quiet {
        println!("{}", message);
    } else {
        Notification::new()
            .summary("File Uploader")
            .body(&message)
            .show()
            .unwrap();
    }

    let rpath = if let Some(path) = args.records_path {
        Path::new(&path).join("records.json")
    } else {
        Path::new(&env::var("CLIENT_DIR").unwrap_or("./".to_string())).join("records.json")
    };

    match upload_files(file_paths, rpath.to_str().unwrap()).await {
        Ok(urls) => {
            ctx.set_contents("".to_owned())
                .expect("Failed to clear clipboard");
            ctx.set_contents(urls.first().unwrap_or(&"".to_owned()).to_owned())
                .expect("Failed to set clipboard contents");
            ctx.get_contents()
                .expect("Failed to get clipboard contents");
            Notification::new()
                .summary("File Uploader")
                .body(&format!("Successfully uploaded {} file[s]", urls.len()))
                .show()
                .unwrap();
        }
        Err(err) => {
            if args.quiet {
                eprintln!("Error: {}", err);
            } else {
                Notification::new()
                    .summary("File Uploader")
                    .body(&format!("File Upload Failed\n{}", err))
                    .show()
                    .unwrap();
            }
        }
    }
}

async fn upload_files(
    file_paths: Vec<PathBuf>,
    records_path: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let base_url = env::var("BASE_URL")?;
    let upload_url = Path::new(&base_url)
        .join("upload")
        .to_string_lossy()
        .to_string();

    let mut fnames = Vec::new();
    let mut form = multipart::Form::new();

    let mprogress = MultiProgress::new();
    let style =
        ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("##-");

    for file_path in file_paths {
        if !file_path.exists() {
            return Err("File path does not exist".into());
        }

        let file_name_str = file_path
            .file_name()
            .and_then(|os_str| os_str.to_str())
            .ok_or("Invalid file name")?
            .to_owned();
        fnames.push(file_name_str.clone());

        let file = File::open(file_path).await?;
        let file_len = file.metadata().await?.len();

        //let fstream = FramedRead::new(file, BytesCodec::new());
        let mut fstream = ReaderStream::new(file); // Use reader stream for progress bar integration

        let progress = mprogress.add(ProgressBar::new(file_len));
        progress.set_style(style.clone());
        let mut uploaded: u64 = 0;

        let async_stream = async_stream::stream! {
            while let Some(chunk) = fstream.next().await {
                if let Ok(chunk) = &chunk {
                    let new = min(uploaded + (chunk.len() as u64), file_len);
                    uploaded = new;
                    progress.set_position(new);
                    if uploaded >= file_len {
                        progress.finish_with_message("Successfully Uploaded");
                    }
                }
                yield chunk;
            }
        };

        let fbody = Body::wrap_stream(async_stream);

        let upload_file = multipart::Part::stream_with_length(fbody, file_len)
            .file_name(file_name_str.clone())
            .mime_str(
                &mime_guess::from_path(&file_name_str)
                    .first()
                    .unwrap_or(mime::TEXT_PLAIN)
                    .to_string(),
            )?;

        form = form.part("file", upload_file);
    }

    let client = Client::new();
    let res = client
        .post(upload_url)
        .header("ACCESS-KEY", env::var("ACCESS_KEY")?)
        .multipart(form)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err("Upload failed".into());
    }

    let text = res.text().await?;
    let file_urls: Vec<&str> = text.split(' ').collect();

    let final_urls: Vec<_> = file_urls
        .iter()
        .zip(fnames.iter())
        .map(|(file_url, fname)| async move {
            let base_url = env::var("BASE_URL").expect("Base Url undefined");
            let final_url = Path::new(&base_url).join("files").join(file_url);

            create_record(
                records_path,
                Record {
                    time: Local::now().format("%m/%d/%Y %H:%M:%S").to_string(),
                    original_file_name: fname.clone(),
                    url_location: final_url.to_string_lossy().to_string(),
                },
            )
            .await
            .unwrap_or_else(|err| eprintln!("Error creating record: {}", err));

            final_url
        })
        .collect();

    let mut ret_urls: Vec<String> = Vec::new();
    for url in final_urls {
        ret_urls.push(url.await.to_string_lossy().to_string());
    }
    Ok(ret_urls)
}
