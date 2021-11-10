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

use inotify::{Inotify, WatchMask, WatchDescriptor, EventMask};
use notify_rust::Notification;

use std::{env, fs};
use std::process::Command;
use std::collections::HashMap;
use std::path::PathBuf;


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

type RuleMap = HashMap<String, Rule>;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    rules: RuleMap
}

#[derive(Serialize, Deserialize, Debug)]
struct Rule {
    path: String,
    args: Vec<String>,
    cmd: String,
    msg: String,
    x: String,
}    

// search for configuration file
// - in the current directory (precedence)
// - in the xdg config directory for scanwatch
// If neither is found, a helpful message will be displayed.
fn read_config_file() -> std::io::Result<String> {

    let mut cwd = env::current_dir().unwrap();
    cwd.push(CONFIG_FILE);
    if cwd.exists() {
        debug!("Found configuration file {}", cwd.to_string_lossy());
        let config_string = fs::read_to_string(cwd)?;
        return Ok(config_string);
    }

    if let Some(mut config_dir) = dirs::config_dir() {
        config_dir.push("scanwatch");
        config_dir.push(CONFIG_FILE);
        if config_dir.exists() {
            debug!("Found configuration file {}", config_dir.to_string_lossy());
            let config_string = fs::read_to_string(config_dir)?;
            return Ok(config_string);
        }
    }

    Err(std::io::Error::new(std::io::ErrorKind::Other, "no configuration file found"))
}

fn main() {
    env_logger::init();

    let config: Config = match read_config_file() {
        Ok(config_string) => {
            toml::from_str(&config_string).unwrap()
        },
        Err(err) => {
            panic!("{}", err.to_string());
        }
    };

    for (key, _rule) in config.rules.iter() {
        debug!("recognized rule: {}", key);
    }
    
    watch_all(&config.rules);
}

fn watch_all(rules: &RuleMap) {
    let mut inotify = Inotify::init()
        .expect("Error while initializing inotify instance");

    // The wdmap is used to identify the rule from the WatchDescriptor
    // that is returned by the Event.
    let mut wdmap: HashMap<WatchDescriptor, String> = HashMap::new();

    // The event_mask_map is used to store the last retrieved event
    // mask for a given file. It is needed to react on
    // events that always come in pairs, such as CREATE/CLOSE_WRITE.
    let mut event_mask_map: HashMap<PathBuf, EventMask> = HashMap::new();
    
    // For now, we only watch CREATE events. If we were watching
    // others, we would need to make sure, that we do not trigger
    // events twice, e.g. a file creation will trigger both CREATE and
    // CLOSE_WRITE events.
    let mask = WatchMask::CREATE;
    
    for (key, rule) in rules.iter() {
        match inotify.add_watch(rule.path.clone(), mask) {
            Ok(wd) => {
                debug!("successfully added watch for rule {}", key);
                wdmap.insert(wd, key.to_string());
            }
            Err(err) => {
                debug!("failed to add watch for rule {}. Error is: {}", key, err.to_string());
            }
        }
    }

    // read events
    let mut buffer = [0; 4096]; // TODO: this number is a black box... is it enough?

    display_notification("Happy Watch!");
    
    loop {
        let events = inotify.read_events_blocking(&mut buffer)
            .expect("Error while reading events");

        for event in events {
            if let Some(key) = wdmap.get(&event.wd) {
                let msg = format!("triggered rule '{}'", key);
                let filename_short = event.name.unwrap().to_string_lossy();
                let is_pdf = filename_short.ends_with(".pdf");
                let rule = rules.get(key).unwrap();

                // this tries to determine the absolute file name
                // it is definitely a mess, and I might be better off using
                // a crate such as path-absolutize. But for now, I will try
                // to stick with this simple implementation.
                let mut file_pathbuf = env::current_dir().unwrap();
                file_pathbuf.push(rule.path.clone());
                file_pathbuf.push(event.name.unwrap());
                let filename = file_pathbuf.to_str().unwrap();
                
                debug!("triggered event for rule {}", key);
                debug!("msg = {}", msg);
                debug!("filename = {}", filename);
                debug!("is_pdf = {}", is_pdf);
                debug!("root path = {}", rule.path);

                if let Some(last_event_mask) = event_mask_map.get(&<>file_pathbuf) {
                    // only trigger on_close action if we have an event
                    // queue, i.e. first CREATE, then CLOSE_WRITE
                    if last_event_mask.contains(EventMask::CREATE) &&
                        event.mask.contains(EventMask::CLOSE_WRITE) &&
                        !event.mask.contains(EventMask::ISDIR) {
                            // TODO: filter for extension, e.g. .pdf
                            info!("triggered rule {}. File: {}", key, filename);

                            let replace_vars = |txt: &'_ str| {
                                txt.replace("{filename}", &filename)
                                    .replace("{filename:short}", &filename_short)
                                    .replace("{x}", &rule.x)
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
                } 
                
                event_mask_map.insert(file_pathbuf, event.mask);
            }
        }
    }
}
