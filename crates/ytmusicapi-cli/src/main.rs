use std::{
    fs,
    io::{self, Read},
    process::ExitCode,
};

fn main() -> ExitCode {
    let mut raw = String::new();
    if let Err(error) = io::stdin().read_to_string(&mut raw) {
        eprintln!("failed to read stdin: {error}");
        return ExitCode::from(1);
    }

    let json = match ytmusicapi::setup_browser_auth(&raw) {
        Ok(json) => json,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    if let Err(error) = fs::write("browser.json", json) {
        eprintln!("failed to write browser.json: {error}");
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}
