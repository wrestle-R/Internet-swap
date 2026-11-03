mod daemon;
mod model;
mod network;

#[cfg(feature = "desktop")]
#[tauri::command]
async fn status() -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(|| daemon::request("status"))
        .await
        .map_err(|e| e.to_string())?
}
#[cfg(feature = "desktop")]
#[tauri::command]
async fn control(action: String) -> Result<serde_json::Value, String> {
    if !["enable", "disable", "test"].contains(&action.as_str())
        && !action
            .strip_prefix("connect ")
            .is_some_and(|s| s.len() == 36 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-'))
    {
        return Err("Unknown action".into());
    }
    tauri::async_runtime::spawn_blocking(move || daemon::request(&action))
        .await
        .map_err(|e| e.to_string())?
}
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "--daemon") {
        if let Err(e) = daemon::serve() {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    if let Some(arg) = args.first() {
        let command = match arg.as_str() {
            "--status" => "status".into(),
            "--enable" => "enable".into(),
            "--disable" => "disable".into(),
            "--test" => "test".into(),
            "--connect" if args.len() == 2 => format!("connect {}", args[1]),
            _ => {
                eprintln!("Unknown option");
                std::process::exit(2)
            }
        };
        match daemon::request(&command) {
            Ok(value) => {
                println!("{value}");
                if value.get("error").is_some() {
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1)
            }
        }
        return;
    }
    #[cfg(feature = "desktop")]
    {
        if daemon::request("status").is_err() {
            let exe = std::env::var_os("APPIMAGE")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::env::current_exe().unwrap());
            let _ = std::process::Command::new(exe)
                .arg("--daemon")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
        tauri::Builder::default()
            .invoke_handler(tauri::generate_handler![status, control])
            .run(tauri::generate_context!())
            .expect("Could not start Internet Swap");
    }
    #[cfg(not(feature = "desktop"))]
    eprintln!("Desktop support was not included in this build");
}
