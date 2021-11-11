///! Very simple app that watches a directory for newly created
///! or overwritten files and sends the file to a printer

// TODO:
// - read configuration from xdg-compatible directory
// - handle more than create events, which is tricky, because we need
//   to keep track of the former events and decide what to do
// - how often does inotify poll? Is there any way to change poll time?
//   Is there any _need_ to do it?


// if you want to see debug output during testing, run via
// RUST_LOG=debug cargo run


use log::{debug, info};
use serde_derive::{Deserialize, Serialize};

use notify_rust::Notification;

use notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent};
use std::sync::mpsc::channel;
use std::time::Duration;

use std::{env, fs};
use std::process::Command;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const CONFIG_FILE: &'static str = "scanwatch.toml";

fn display_notification(message: &'_ str) {
    // if below notification fails, then this is not at all fatal, as
    // we print out the message on the command line
    println!("»»» {}", message);

    Notification::new()
        .summary("scanwatch")
        .body(message)
        .icon("printer")
        .show()
        .unwrap();
}

// very simple error type: just an error message
type SwResult<T> = Result<T, String>;

type RuleMap = HashMap<String, Rule>;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    rules: RuleMap,
    path: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Rule {
    args: Vec<String>,
    cmd: String,
    msg: String,
    #[serde(default)] x: String,
    #[serde(default)] y: String,
    #[serde(default)] z: String
}    

fn main() {
    env_logger::init();
    if let Ok(config) = read_config() {
        watch_all(&config);
    }
}

// search for configuration file
// - in the current directory (precedence)
// - in the xdg config directory for scanwatch
// If neither is found, a helpful message will be displayed.
fn read_config() -> SwResult<Config> {

    let mut cwd = env::current_dir().unwrap();
    cwd.push(CONFIG_FILE);

    if cwd.exists() {
        debug!("Found configuration file {}", cwd.to_string_lossy());
        let config_string = fs::read_to_string(cwd)
            .map_err(|err| "io error".to_string())?;
        return toml::from_str(&config_string)
            .map_err(|err| "toml error".to_string());
    }

    if let Some(mut config_dir) = dirs::config_dir() {
        config_dir.push("scanwatch");
        config_dir.push(CONFIG_FILE);
        if config_dir.exists() {
            debug!("Found configuration file {}", config_dir.to_string_lossy());
            let config_string = fs::read_to_string(config_dir)
                .map_err(|err| "io error".to_string())?;
            return toml::from_str(&config_string)
                .map_err(|err| "toml error".to_string());
        }
    }

    Err(String::from("no configuration file found"))
}


fn watch_all(config: &Config) -> notify::Result<()> {
    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2))?;

    // construct absolute path and start watching
    let mut pb = std::env::current_dir()?;
    pb.push(config.path.clone());
    watcher.watch(config.path.clone(), RecursiveMode::Recursive)?;
    debug!("watching path {}", pb.to_string_lossy());

    display_notification("Happy Watch!");

    loop {
        match rx.recv() {
            Ok(event) => {
                println!("received event: {:?}", event);
                match event {
                    DebouncedEvent::Create(pb) | DebouncedEvent::Write(pb) => config.rules.iter().for_each(|(_key, rule)| exec_rule(&rule, pb.clone())),
                    _ => debug!("unhandled event: {:?}", event),
                }
            },
            Err(e) => debug!("watch error: {:?}", e)
        }
    }
}

fn exec_rule(rule: &Rule, matched_path: PathBuf) {
    let filename = matched_path.to_string_lossy();
    let filename_short = matched_path.file_name().unwrap().to_string_lossy();
    
    let replace_vars = |txt: &'_ str| {
        txt.replace("{filename}", &filename)
            .replace("{filename:short}", &filename_short)
            .replace("{x}", &rule.x)
            .replace("{y}", &rule.y)
            .replace("{z}", &rule.z)
    };
                        
    let msg = replace_vars(&rule.msg);
    let args = rule.args.iter().map(|arg| replace_vars(arg)).collect::<Vec<String>>();

    debug!("msg = {}", msg);
    for arg in &args {
        debug!("arg = {}", arg);
    }

    // TODO: unwrap() may be difficult here, because
    // the user's template might fail
    display_notification(&msg);
                            
    let _child =
        Command::new(&rule.cmd)
        .args(args)
        .spawn()
        .expect("failed to execute process");
}
