use log::{info, error, debug};
use serde_derive::Deserialize;

#[derive(Deserialize)]
struct Config {
    path: String
}

fn main() {
    env_logger::init();
    
    let config: Config = toml::from_str(r#"
        path = './test'
"#).unwrap();

    info!("Watching path {path}", path=config.path);

    
}
