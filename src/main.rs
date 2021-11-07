///! Very simple app that watches a directory for newly created
///! or overwritten files and sends the file to a printer

// TODO:
// - read configuration from xdg-compatible directory
// - allow multiple directories by specifying multiple config sections
//   [path]

// if you want to see debug output during testing, run via
// RUST_LOG=debug cargo run

use log::{error, debug};
use serde_derive::{Deserialize, Serialize};

use notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent};

use notify_rust::Notification;

use std::sync::mpsc::channel;
use std::time::Duration;
use std::{env, fs};
use std::io::{self, Write};
use std::process::Command;
use std::collections::HashMap;

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


#[derive(Serialize, Deserialize, Debug)]
struct Config {
    rule: HashMap<String, Rule>
}

#[derive(Serialize, Deserialize, Debug)]
struct Rule {
    path: String,
    args: Vec<String>,
    cmd: String,
    msg: String,
    x: String,
}


fn watch(rule: &Rule) -> notify::Result<()> {

    notify(&format!("Watching path {path} for incoming documents (pdf only)",
                    path=rule.path));

    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher =
        Watcher::new(tx, Duration::from_secs(2))?;

    watcher.watch(&rule.path, RecursiveMode::Recursive)?;

    loop {
        match rx.recv() {
            Ok(DebouncedEvent::Write(pathbuf)) |
            Ok(DebouncedEvent::Create(pathbuf)) => {
                // TODO: could be easily written using the nightly-feature `contains`
                // let is_pdf = pathbuf.as_path().extension().contains("pdf");
                match pathbuf.as_path().extension() {
                    Some(ext) if ext == "pdf" => {

                        let filename = pathbuf.to_string_lossy();
                        let filename_short = pathbuf.as_path().file_name().unwrap().to_string_lossy();

                        let replace_vars = |txt: &'_ str| {
                            txt.replace("{filename}", &filename)
                                .replace("{filename:short}", &filename_short)
                                .replace("{x}", &rule.x)
                        };
                        
                        let msg = replace_vars(&rule.msg);
                        let args = rule.args.iter().map(|arg| replace_vars(arg)).collect::<Vec<String>>();
                        
                        println!("msg => {}", msg);
                        for arg in &args {
                            println!("  arg => {}", arg);
                        }

                        // TODO: unwrap() may be difficult here, because the user's template might fail
                        notify(&msg);
                        
                        let _child =
                            Command::new(&rule.cmd)
                            .args(args)
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

    for (key, rule) in config.rule.iter() {
        debug!("recognized rule: {}", key);
    }
    
    // use first rule and skip the others (for the moment)
    if let Some((key, rule)) = config.rule.iter().next() {
        if let Err(e) = watch(&rule) {
            error!("error: {:?}", e);
        }
    }
}
