//! `ss-release-sign` — release signing for CI and the maintainer (ADR 0006 §3,
//! `docs/architecture/release.md`).
//!
//! * `keygen <secret-file>` writes a new private key to `<secret-file>` and prints the public key.
//! * `sign <package>…` signs each package with `$UPDATE_SIGNING_KEY` into `<package>.sig`.
//! * `verify <package>…` checks each package against the public key built into the app.

use std::path::Path;
use std::process::ExitCode;

use ss_update::SIGNATURE_SUFFIX;
use ss_update::signature::{generate_secret, public_key_of, sign, verify};

/// Environment variable with the base64 private key.
const SECRET_ENV: &str = "UPDATE_SIGNING_KEY";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.split_first() {
        Some((command, [file])) if command == "keygen" => keygen(Path::new(file)),
        Some((command, files)) if command == "sign" && !files.is_empty() => sign_all(files),
        Some((command, files)) if command == "verify" && !files.is_empty() => verify_all(files),
        _ => Err(
            "usage: ss-release-sign keygen <secret-file> | sign <package>… | verify <package>…"
                .to_owned(),
        ),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn keygen(secret_file: &Path) -> Result<(), String> {
    if secret_file.exists() {
        return Err(format!(
            "{} exists; not overwriting a key",
            secret_file.display()
        ));
    }
    let secret = generate_secret().map_err(|e| format!("random source: {e}"))?;
    let public = public_key_of(&secret).map_err(|e| e.to_string())?;
    std::fs::write(secret_file, format!("{secret}\n"))
        .map_err(|e| format!("writing {}: {e}", secret_file.display()))?;
    println!("{public}");
    Ok(())
}

fn sign_all(files: &[String]) -> Result<(), String> {
    let secret = std::env::var(SECRET_ENV).map_err(|_| format!("{SECRET_ENV} is not set"))?;
    for file in files {
        let (name, bytes) = read(file)?;
        let signature = sign(&secret, &name, &bytes).map_err(|e| e.to_string())?;
        let target = format!("{file}{SIGNATURE_SUFFIX}");
        std::fs::write(&target, format!("{signature}\n"))
            .map_err(|e| format!("writing {target}: {e}"))?;
        println!("signed {name}");
    }
    Ok(())
}

fn verify_all(files: &[String]) -> Result<(), String> {
    for file in files {
        let (name, bytes) = read(file)?;
        let target = format!("{file}{SIGNATURE_SUFFIX}");
        let signature =
            std::fs::read_to_string(&target).map_err(|e| format!("reading {target}: {e}"))?;
        verify(&name, &bytes, &signature).map_err(|e| format!("{name}: {e}"))?;
        println!("verified {name}");
    }
    Ok(())
}

fn read(file: &str) -> Result<(String, Vec<u8>), String> {
    let path = Path::new(file);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| format!("{file} is not a file"))?;
    let bytes = std::fs::read(path).map_err(|e| format!("reading {file}: {e}"))?;
    Ok((name, bytes))
}
