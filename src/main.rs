#[macro_use]
extern crate log;

mod api;
mod models;

use env_logger;
use models::Config;

use std::fs;
use std::io::{Read, Write};
use toml;
use xdg;

fn main() {
    env_logger::init();

    let config_path = {
        let xdg_dirs = xdg::BaseDirectories::with_prefix("simple-library-explorer").unwrap();
        xdg_dirs
            .place_config_file("config.toml")
            .expect("cannot create config directory")
    };

    if !config_path.as_path().exists() {
        debug!("create config file at {:?}", config_path);
        let mut config_file = fs::File::create(&config_path).unwrap();
        let config_str = {
            let default_config: Config = Default::default();
            toml::to_string_pretty(&default_config).unwrap()
        };
        config_file.write_all(config_str.as_bytes()).unwrap();
    }

    debug!("config file at {:?}", config_path);
    let config: Config = {
        let mut config_file = fs::File::open(&config_path).unwrap();
        let mut config_str = String::new();
        config_file.read_to_string(&mut config_str).unwrap();

        toml::from_str(config_str.as_str()).unwrap()
    };

    debug!("API URL: {}", config.api_url.as_str());
    debug!("API KEY: {}", "*".repeat(config.api_key.len()));
    debug!("SYSTEMID: {}", config.systemid);
    debug!("ISBN LIST: {:?}", config.isbn);

    let books = api::fetch_books_status(config).expect("failed to fetch books status.");

    for book in books {
        println!("[{}]", book.isbn);
        for library in book.libraries {
            println!("  {}: {}", library.0, library.1);
        }
    }
}
