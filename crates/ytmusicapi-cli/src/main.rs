use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
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

    if let Err(error) = write_browser_json(Path::new("browser.json"), &json) {
        eprintln!("failed to write browser.json: {error}");
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

#[cfg(unix)]
fn write_browser_json(path: &Path, json: &str) -> io::Result<()> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "browser.json path must include a file name",
        )
    })?;
    let file_name = file_name.to_string_lossy();

    for attempt in 0..1024 {
        let temp_path = parent.join(format!(
            ".{file_name}.tmp.{}.{}",
            std::process::id(),
            attempt
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp_path)
        {
            Ok(mut file) => {
                let result = (|| {
                    file.write_all(json.as_bytes())?;
                    file.sync_all()?;
                    drop(file);
                    fs::rename(&temp_path, path)?;
                    fs::set_permissions(path, fs::Permissions::from_mode(0o644))
                })();

                if result.is_err() {
                    let _ = fs::remove_file(&temp_path);
                }

                return result;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "failed to allocate a unique browser.json temp file",
    ))
}

#[cfg(not(unix))]
fn write_browser_json(path: &Path, json: &str) -> io::Result<()> {
    fs::write(path, json)
}
