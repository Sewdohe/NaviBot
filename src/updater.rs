use std::io::Cursor;
use tokio::sync::mpsc::UnboundedSender;
use crate::types::{BotEvent, LogLevel};

// ──────────────────────────────────────────────
// Semver helpers
// ──────────────────────────────────────────────

fn parse_semver(s: &str) -> (u64, u64, u64) {
    // strip leading 'v', drop pre-release suffix (e.g. "-alpha.1")
    let s = s.trim_start_matches('v');
    let s = s.split('-').next().unwrap_or(s);
    let mut parts = s.splitn(3, '.');
    let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

fn is_newer(latest: &str, current: &str) -> bool {
    parse_semver(latest) > parse_semver(current)
}

// ──────────────────────────────────────────────
// Platform helpers
// ──────────────────────────────────────────────

fn asset_name() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    { "navi-linux-arm64.zip" }
    #[cfg(all(target_os = "linux", not(target_arch = "aarch64")))]
    { "navi-linux.zip" }
    #[cfg(target_os = "windows")]
    { "navi-windows.zip" }
    #[cfg(target_os = "macos")]
    { "navi-macos.zip" }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    { "navi-linux.zip" }
}

fn binary_name() -> &'static str {
    #[cfg(target_os = "windows")]
    { "navi_bot.exe" }
    #[cfg(not(target_os = "windows"))]
    { "navi_bot" }
}

// ──────────────────────────────────────────────
// Core update flow
// ──────────────────────────────────────────────

pub async fn check_and_update(tx: UnboundedSender<BotEvent>) {
    let log = |level: LogLevel, msg: &str| {
        let _ = tx.send(BotEvent::Log(level, msg.to_string()));
    };

    log(LogLevel::Info, "Checking for updates...");

    let client = match reqwest::Client::builder()
        .user_agent(concat!("NaviBot/", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log(LogLevel::Error, &format!("Failed to build HTTP client: {}", e));
            return;
        }
    };

    // 1. Fetch latest release from GitHub API
    let resp = match client
        .get("https://api.github.com/repos/Sewdohe/NaviBot/releases/latest")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log(LogLevel::Error, &format!("Update check failed: {}", e));
            return;
        }
    };

    let json: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            log(LogLevel::Error, &format!("Failed to parse release JSON: {}", e));
            return;
        }
    };

    let tag = match json["tag_name"].as_str() {
        Some(t) => t,
        None => {
            log(LogLevel::Error, "GitHub response missing tag_name field");
            return;
        }
    };

    let current = env!("CARGO_PKG_VERSION");
    log(LogLevel::Info, &format!("Current: v{}  Latest: {}", current, tag));

    if !is_newer(tag, current) {
        log(LogLevel::Info, "Already up to date.");
        return;
    }

    // 2. Find the correct asset URL
    let assets = match json["assets"].as_array() {
        Some(a) => a,
        None => {
            log(LogLevel::Error, "No assets found in release");
            return;
        }
    };

    let target = asset_name();
    let asset_url = assets.iter().find_map(|a| {
        let name = a["name"].as_str()?;
        if name == target {
            a["browser_download_url"].as_str().map(|s| s.to_string())
        } else {
            None
        }
    });

    let asset_url = match asset_url {
        Some(url) => url,
        None => {
            log(LogLevel::Error, &format!("No asset named '{}' found in release {}", target, tag));
            return;
        }
    };

    log(LogLevel::Info, &format!("Downloading {} ...", asset_url));

    // 3. Download the zip
    let zip_bytes = match client.get(&asset_url).send().await {
        Ok(r) => match r.bytes().await {
            Ok(b) => b.to_vec(),
            Err(e) => {
                log(LogLevel::Error, &format!("Failed to read download body: {}", e));
                return;
            }
        },
        Err(e) => {
            log(LogLevel::Error, &format!("Download failed: {}", e));
            return;
        }
    };

    log(LogLevel::Info, &format!("Downloaded {} bytes. Extracting...", zip_bytes.len()));

    // 4. Extract & replace (blocking I/O inside block_in_place)
    let bin_name = binary_name();
    let tx_clone = tx.clone();
    let result = tokio::task::block_in_place(|| {
        extract_and_replace(zip_bytes, bin_name, &tx_clone)
    });

    match result {
        Ok(()) => {
            log(LogLevel::Info, "Binary replaced successfully. Restarting...");
            // Small delay so the TUI can render the message before exec
            std::thread::sleep(std::time::Duration::from_millis(400));
            restart_process();
        }
        Err(e) => {
            log(LogLevel::Error, &format!("Update failed during extraction: {}", e));
        }
    }
}

// ──────────────────────────────────────────────
// Synchronous helpers (called via block_in_place)
// ──────────────────────────────────────────────

fn extract_and_replace(
    zip_bytes: Vec<u8>,
    binary_name: &str,
    tx: &UnboundedSender<BotEvent>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cursor = Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        // Match the binary by exact name or by path suffix
        let is_match = name == binary_name
            || name.ends_with(&format!("/{}", binary_name))
            || name.ends_with(&format!("\\{}", binary_name));

        if !is_match {
            continue;
        }

        let mut data = Vec::with_capacity(file.size() as usize);
        std::io::copy(&mut file, &mut data)?;

        let current_exe = std::env::current_exe()?;
        let tmp_path = current_exe.with_extension("tmp_update");

        std::fs::write(&tmp_path, &data)?;

        // self_replace handles Windows file-locking and Unix atomicity transparently
        self_replace::self_replace(&tmp_path)?;
        std::fs::remove_file(&tmp_path).ok();

        let _ = tx.send(BotEvent::Log(LogLevel::Info, format!("Replaced binary at {}", current_exe.display())));
        return Ok(());
    }

    Err(format!("'{}' not found inside the downloaded zip", binary_name).into())
}

fn restart_process() -> ! {
    let exe = std::env::current_exe().expect("Could not resolve current exe path");

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(&exe).exec();
        // exec() only returns on error
        eprintln!("exec() failed: {}", err);
        std::process::exit(1);
    }

    #[cfg(not(unix))]
    {
        // Windows: can't rename a running exe, but we spawn then exit
        let _ = std::process::Command::new(&exe).spawn();
        std::process::exit(0);
    }
}

// ──────────────────────────────────────────────
// Unit tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_newer_patch() {
        assert!(is_newer("v1.0.11", "1.0.10"));
    }

    #[test]
    fn semver_older_patch() {
        assert!(!is_newer("v1.0.9", "1.0.10"));
    }

    #[test]
    fn semver_equal() {
        assert!(!is_newer("v1.0.10", "1.0.10"));
    }

    #[test]
    fn semver_major_bump() {
        assert!(is_newer("v2.0.0", "1.9.9"));
    }

    #[test]
    fn semver_no_v_prefix() {
        assert!(is_newer("2.0.0", "1.9.9"));
    }

    #[test]
    fn semver_prerelease_stripped() {
        // "1.0.11-alpha.1" should still beat "1.0.10"
        assert!(is_newer("v1.0.11-alpha.1", "1.0.10"));
    }
}
