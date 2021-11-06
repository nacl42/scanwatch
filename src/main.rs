///! Very simple app that watches a directory for newly created
///! or overwritten files and sends the file to a printer

// TODO:
// - only send pdf files
// - read configuration from xdg-compatible directory
// -

use log::{error};
use serde_derive::{Deserialize, Serialize};

use notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent};

use notify_rust::Notification;

use std::sync::mpsc::channel;
use std::time::Duration;
use std::{env, fs};
use std::io::{self, Write};
use std::process::Command;

const CONFIG_FILE: &'static str = "scanwatch.toml";

fn notify(message: &'_ str) {
    // if below notification fails, then this is not at all fatal, as
    // we print out the message on the command line
    println!("{}", message);

    Notification::new()
        .summary("scanwatch")
        .body(message)
        .icon("printer")
        .show()
        .unwrap();
}


#[derive(Serialize, Deserialize)]
struct Config {
    path: String,
    printer: String,
}

fn watch(config: &Config) -> notify::Result<()> {
    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher =
        Watcher::new(tx, Duration::from_secs(2))?;

    watcher.watch(&config.path, RecursiveMode::Recursive)?;

    loop {
        match rx.recv() {
            Ok(DebouncedEvent::Write(pathbuf)) |
            Ok(DebouncedEvent::Create(pathbuf)) => {
                // TODO: could be easily written using the nightly-feature `contains`
                // let is_pdf = pathbuf.as_path().extension().contains("pdf");
                match pathbuf.as_path().extension() {
                    Some(ext) if ext == "pdf" => {
                        let short_name = pathbuf.as_path().file_name().unwrap().to_string_lossy();
                        notify(&format!("sending document '{}' to printer '{}'", short_name, config.printer));
                        
                        let _child =
                            Command::new("lpr")
                            .args([format!("-P{}", config.printer),
                                   format!("{}", pathbuf.to_string_lossy())])
                            .spawn()
                            .expect("failed to execute process");
                    }
                    _ => { println!("skipped, because it is not a pdf file")}
                };
            }
            Ok(event) => println!("unspecified event: {:?}", event),
            Err(e) => error!("watch error: {:?}", e),
        }
    }
    
}


fn main() {
    env_logger::init();

    let config_string = fs::read_to_string(CONFIG_FILE)
        .expect(&format!("cannot find configuration file {filename}",
                filename=CONFIG_FILE));
    
    let config: Config = toml::from_str(&config_string).unwrap();

    notify(&format!("Watching path {path} for incoming documents (pdf only)",
                    path=config.path));
        
    if let Err(e) = watch(&config) {
        error!("error: {:?}", e);
    }
}
