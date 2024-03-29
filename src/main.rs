///! Very simple app that watches a directory for newly created
///! or overwritten files and sends the file to a printer

// if you want to see debug output during testing, run via
// RUST_LOG=debug cargo run

// TODO:
// - get rid of warning messages
// - document functions
// - document configuration in README.md
// - maybe have an option to quit rule evaluation if a rule
// matches. This would require the rules to be run in the order they
// appear.
// - make sound when rule is triggered
// - count pages of pdf and match rule only if given number of pages
// is higher or lower than given value.

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
use std::ffi::{OsStr, OsString};


const CONFIG_FILE: &'static str = "scanwatch.toml";

fn display_notification(message: &'_ str, icon: &Option<String>) {
    // if below notification fails, then this is not at all fatal, as
    // we print out the message on the command line
    debug!("{}", message);
    println!("»»» {}", message);
    let title = "scanwatch";
    
    if let Some(icon) = icon {
        Notification::new().summary(title).body(message).icon(&icon).show().unwrap();
    } else {
        Notification::new().summary(title).body(message).show().unwrap();
    }
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
    icon: Option<String>,
    ends_with: Option<String>,
    starts_with: Option<String>,
    filter: Option<String>,
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
    pb.push(expand_tilde(&config.path));
    watcher.watch(pb.as_path().clone(), RecursiveMode::Recursive)
        .expect(&format!("failed to watch path '{}', maybe it does not exist...?", pb.to_string_lossy()));

    display_notification(
        &format!("watching path '{}'", pb.to_string_lossy()),
        &None
    );

    loop {
        match rx.recv() {
            Ok(event) => {
                println!("received event: {:?}", event);
                match event {
                    DebouncedEvent::Create(pb) | DebouncedEvent::Write(pb) =>
                        config.rules.iter().for_each(|(_key, rule)| exec_rule(&rule, pb.clone())),
                    _ => debug!("unhandled event: {:?}", event),
                }
            },
            Err(e) => debug!("watch error: {:?}", e)
        }
    }
}

fn exec_rule(rule: &Rule, matched_path: PathBuf) {

    let filename = matched_path.to_string_lossy();

    // filename_short is the stripped version without the base path
    let filename_short = matched_path.file_name().unwrap().to_string_lossy();

    // if 'ends_with' or 'starts_with' are given, then the corresponding
    // parts of the short filename must match
    if let Some(ends_with) = &rule.ends_with {
        if !filename_short.ends_with(ends_with) {
            debug!("filename does not end with '{}', skipping rule", ends_with);
            return;
        }
    }

    if let Some(starts_with) = &rule.starts_with {
        if !filename_short.starts_with(starts_with) {
            debug!("filename '{}' does not start with '{}', skipping rule", filename_short, starts_with);
            return;
        }
    }
    
    // if a filter exists, the short filename must match
    if let Some(filter) = &rule.filter {
        debug!("matching {} against filter {}", filename_short, filter);
        if let Ok(re) = regex::Regex::new(&filter) {
            debug!("successfully compiled regular expression");
            if re.is_match(&filename_short) {
                debug!("filter matches");
            } else {
                debug!("not a match at all, skipping rule!");
                return;
            }
        };
    };
    
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
    display_notification(&msg, &rule.icon);
                            
    let _child =
        Command::new(&rule.cmd)
        .args(args)
        .spawn()
        .expect("failed to execute process");
}



fn expand_tilde<P: AsRef<Path>>(path: P) -> PathBuf {
    let home_dir = dirs::home_dir().unwrap().into_os_string();

    let mut result = PathBuf::new();
    path.as_ref()
        .components()
        .map(|c| c.as_os_str())
        .map(|c| if c == "~" { home_dir.clone() } else { OsString::from(c) })
        .for_each(|oss| result.push(oss));

    result
}

