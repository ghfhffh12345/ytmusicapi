use std::{
    fs,
    io::{self, Read, Write},
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

    if let Err(error) = write_browser_json("browser.json", &json) {
        eprintln!("failed to write browser.json: {error}");
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

#[cfg(unix)]
fn write_browser_json(path: &str, json: &str) -> io::Result<()> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(json.as_bytes())?;
    file.set_permissions(fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn write_browser_json(path: &str, json: &str) -> io::Result<()> {
    fs::write(path, json)
}
