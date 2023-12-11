use std::{env, process::Command};
mod urlhandler;

fn main() {
    let mut config_path = env::current_exe().expect("msg").clone();
    config_path.set_file_name("config.json");
    std::println!("Loading config from: {}", config_path.display());

    let config_string = std::fs::read_to_string(config_path).expect("Failed to read config");
    let config = json::parse(&config_string).expect("Failed to parse config");

    let browsers : &json::JsonValue = &config["browsers"];
    if browsers.is_null() {
        panic!("Browsers config section does not exist");
    }

    let rules : &json::JsonValue = &config["rules"];
    if rules.is_null() {
        panic!("Browsers config section does not exist");
    }

    let args : Vec<String> = env::args().collect();
    if args.len() < 2 {
        urlhandler::register().expect("Failed to register");
        return;
    }

    for rule in rules.entries() {
        let url = args.get(1).expect("Missing url arg");
        if url.starts_with(rule.0) {
            let browser_exec = browsers[rule.1.to_string()].to_string();

            println!("Using {browser_exec} for {url}", );

            Command::new(browser_exec)
                     .arg(url)
                     .spawn()
                     .expect("Failed to execute command");

            break;
        }
    }
}





