use log::{error};
use serde_derive::Deserialize;
use notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::{env, fs};
use std::io::{self, Write};
use std::process::Command;

const CONFIG_FILE: &'static str = "scanwatch.toml";

#[derive(Deserialize)]
struct Config {
    path: String
}

fn watch(config: &Config) -> notify::Result<()> {
    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher =
        Watcher::new(tx, Duration::from_secs(2))?;

    watcher.watch(&config.path, RecursiveMode::Recursive)?;

    loop {
        match rx.recv() {
            Ok(DebouncedEvent::Write(pathbuf)) => {
                println!("Write to {}", pathbuf.to_string_lossy());
                let output =
                    Command::new("ls")
                    //.arg("ls")
                    .output()
                    .expect("failed to execute process");

                io::stdout().write_all(&output.stdout).unwrap();
                println!("---");
            }
            Ok(event) => println!("{:?}", event),
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
