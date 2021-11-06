use log::{error};
use serde_derive::{Deserialize, Serialize};
use notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::{env, fs};
use std::io::{self, Write};
use std::process::Command;
use tinytemplate::TinyTemplate;

const CONFIG_FILE: &'static str = "scanwatch.toml";

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
            Ok(DebouncedEvent::Write(pathbuf)) | Ok(DebouncedEvent::Create(pathbuf))|Ok(DebouncedEvent::NoticeWrite(pathbuf)) => {
                println!("» printing document '{}' to printer '{}'", pathbuf.to_string_lossy(), config.printer);
                let output =
                    Command::new("echo")
                    .arg(format!("-P {} {}",
                                 config.printer,
                                 pathbuf.to_string_lossy()))
                    .output()
                    .expect("failed to execute process");

                io::stdout().write_all(&output.stdout).unwrap();
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

    println!("Watching path {path}", path=config.path);

    if let Err(e) = watch(&config) {
        error!("error: {:?}", e);
    }
}
