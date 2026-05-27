use std::process::Command;

fn main() {
    // Git commit hash
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Build timestamp (ISO 8601)
    let build_time = chrono_now()
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_HASH={git_hash}");
    println!("cargo:rustc-env=BUILD_TIME={build_time}");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");
}

fn chrono_now() -> Option<String> {
    // Use the system date command for portability, avoiding chrono dep in build.rs
    let output = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()?;
    if output.status.success() {
        String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
    } else {
        None
    }
}
