use log::{info, error, debug, log};
use serde_derive::Deserialize;
use notify::{RecommendedWatcher, Watcher, RecursiveMode};
use std::sync::mpsc::channel;
use std::time::Duration;

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
            Ok(event) => println!("{:?}", event),
            Err(e) => error!("watch error: {:?}", e),
        }
    }
    
}


fn main() {
    env_logger::init();
    
    let config: Config = toml::from_str(r#"
        path = './test'
"#).unwrap();

    println!("Watching path {path}", path=config.path);
    info!("Watching path {path}", path=config.path);

    if let Err(e) = watch(&config) {
        error!("error: {:?}", e);
    }
}
