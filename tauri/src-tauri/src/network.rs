use crate::model::*;
use std::{
    collections::HashMap,
    process::{Command, Stdio},
};

/// AppImage library paths belong to the GUI, never to host NetworkManager/curl.
pub fn host_command(program: &str) -> Command {
    let mut command = Command::new(program);
    for key in [
        "LD_LIBRARY_PATH",
        "LD_PRELOAD",
        "GTK_PATH",
        "GIO_EXTRA_MODULES",
        "GDK_PIXBUF_MODULE_FILE",
        "GI_TYPELIB_PATH",
        "PYTHONHOME",
        "PYTHONPATH",
        "PERL5LIB",
        "QT_PLUGIN_PATH",
        "QT_QPA_PLATFORM_PLUGIN_PATH",
    ] {
        command.env_remove(key);
    }
    if let (Some(bundle), Some(path)) = (std::env::var_os("APPDIR"), std::env::var_os("PATH")) {
        let bundle = std::path::PathBuf::from(bundle);
        let paths = std::env::split_paths(&path)
            .filter(|p| !p.starts_with(&bundle))
            .collect::<Vec<_>>();
        if let Ok(path) = std::env::join_paths(paths) {
            command.env("PATH", path);
        }
    }
    command
}

pub fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let output = host_command("timeout")
        .args(["--kill-after=2", "40", program])
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "{}: {}",
            program,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
pub fn nm(args: &[&str]) -> Result<String, String> {
    run("nmcli", args)
}
pub fn interface() -> Result<String, String> {
    nm(&["-t", "-f", "DEVICE,TYPE", "device", "status"])?
        .lines()
        .filter_map(|l| {
            let f = fields(l);
            if f.len() == 2 && f[1] == "wifi" {
                Some(f[0].clone())
            } else {
                None
            }
        })
        .next()
        .ok_or("No Wi-Fi adapter found".into())
}
pub fn current(iface: &str) -> Result<String, String> {
    let s = nm(&["-g", "GENERAL.CON-UUID", "device", "show", iface])?;
    Ok(if s == "--" { String::new() } else { s })
}
pub fn scan(iface: &str, old: &[Network]) -> Result<Vec<Network>, String> {
    // Only profiles with saved credentials are eligible; never request or export passwords.
    let profiles = nm(&["-t", "-f", "UUID,TYPE", "connection", "show"])?;
    let visible = nm(&[
        "-t",
        "-f",
        "SSID,SIGNAL,FREQ,SECURITY",
        "device",
        "wifi",
        "list",
        "ifname",
        iface,
        "--rescan",
        "auto",
    ])?;
    let active = current(iface)?;
    let mut access: HashMap<String, (u8, String)> = HashMap::new();
    for row in visible.lines() {
        let f = fields(row);
        if f.len() != 4 || f[0].is_empty() {
            continue;
        }
        // Saved open profiles remain visible in NetworkManager, but are not auto-selected.
        if f[3].is_empty() || f[3] == "--" {
            continue;
        }
        let signal = f[1].parse::<u8>().unwrap_or(0);
        let freq = f[2]
            .split_whitespace()
            .next()
            .unwrap_or("")
            .parse::<u32>()
            .unwrap_or(0);
        let band = if freq >= 5925 {
            "6 GHz"
        } else if freq >= 4900 {
            "5 GHz"
        } else if freq > 0 {
            "2.4 GHz"
        } else {
            "Wi-Fi"
        };
        if access.get(&f[0]).is_none_or(|v| signal > v.0) {
            access.insert(f[0].clone(), (signal, band.into()));
        }
    }
    let mut list = vec![];
    for line in profiles.lines() {
        let f = fields(line);
        if f.len() != 2 || f[1] != "802-11-wireless" {
            continue;
        }
        let details = nm(&[
            "-g",
            "connection.id,802-11-wireless.ssid",
            "connection",
            "show",
            "uuid",
            &f[0],
        ])?;
        let mut values = details.lines();
        let name = values.next().unwrap_or("").to_string();
        let ssid = values.collect::<Vec<_>>().join("\n");
        // -g also escapes colons/backslashes; decode each returned value.
        let name = fields(&name).join(":");
        let ssid = fields(&ssid).join(":");
        if let Some((signal, band)) = access.get(&ssid) {
            list.push(Network {
                uuid: f[0].clone(),
                name,
                ssid,
                signal: *signal,
                band: band.clone(),
                active: active == f[0],
                measurement: old
                    .iter()
                    .find(|n| n.uuid == f[0])
                    .and_then(|n| n.measurement.clone()),
            });
        }
    }
    list.sort_by(|a, b| {
        b.active
            .cmp(&a.active)
            .then(b.signal.cmp(&a.signal))
            .then(a.uuid.cmp(&b.uuid))
    });
    Ok(list)
}
pub fn activate(uuid: &str, iface: &str) -> Result<(), String> {
    nm(&[
        "--wait",
        "15",
        "connection",
        "up",
        "uuid",
        uuid,
        "ifname",
        iface,
    ])
    .map(|_| ())
}
pub fn disconnect(iface: &str) -> Result<(), String> {
    nm(&["--wait", "10", "device", "disconnect", iface]).map(|_| ())
}
pub fn health(iface: &str) -> Option<f64> {
    // HTTPS certificate checks stay enabled. Two unrelated endpoints reduce false failovers.
    for (url, expected) in [
        ("https://www.gstatic.com/generate_204", "204"),
        ("https://cp.cloudflare.com/generate_204", "204"),
    ] {
        if let Ok(s) = run(
            "curl",
            &[
                "--interface",
                iface,
                "--silent",
                "--show-error",
                "--connect-timeout",
                "2",
                "--max-time",
                "3",
                "--output",
                "/dev/null",
                "--write-out",
                "%{http_code} %{time_total}",
                url,
            ],
        ) {
            let f = s.split_whitespace().collect::<Vec<_>>();
            if f.len() == 2 && f[0] == expected {
                return f[1].parse::<f64>().ok().map(|s| s * 1000.);
            }
        }
    }
    None
}
pub fn measure(iface: &str) -> Measurement {
    let latency = health(iface);
    let mut m = Measurement {
        at: now(),
        latency,
        healthy: latency.is_some(),
        ..Default::default()
    };
    if !m.healthy {
        return m;
    }
    for upload in [false, true] {
        let mut args = vec![
            "--interface",
            iface,
            "--silent",
            "--show-error",
            "--connect-timeout",
            "3",
            "--max-time",
            "8",
            "--output",
            "/dev/null",
            "--write-out",
            "%{http_code} %{speed_download} %{speed_upload}",
        ];
        let result = if upload {
            // 256 KiB request body, generated in memory and passed on stdin, never a secret.
            use std::io::Write;
            let mut child = host_command("curl")
                .args([
                    "--interface",
                    iface,
                    "--silent",
                    "--show-error",
                    "--connect-timeout",
                    "3",
                    "--max-time",
                    "8",
                    "--output",
                    "/dev/null",
                    "--write-out",
                    "%{http_code} %{speed_upload}",
                    "--request",
                    "POST",
                    "--data-binary",
                    "@-",
                    "https://speed.cloudflare.com/__up",
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn();
            match child.as_mut() {
                Ok(c) => {
                    let sent = c.stdin.take().unwrap().write_all(&vec![0u8; 262144]);
                    if sent.is_err() {
                        let _ = c.kill();
                    }
                    let o = child.unwrap().wait_with_output();
                    o.ok()
                        .filter(|o| o.status.success())
                        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                }
                Err(_) => None,
            }
        } else {
            args.push("https://speed.cloudflare.com/__down?bytes=1048576");
            run("curl", &args).ok()
        };
        if let Some(s) = result {
            let f = s.split_whitespace().collect::<Vec<_>>();
            if f.first() == Some(&"200") {
                let speed = f
                    .get(1)
                    .and_then(|x| x.parse::<f64>().ok())
                    .filter(|v| v.is_finite() && *v > 0.);
                let mbps = speed.map(|v| v * 8. / 1e6);
                if upload {
                    m.upload = mbps;
                } else {
                    m.download = mbps;
                }
            }
        }
    }
    m
}
pub fn counters(iface: &str) -> Option<(u64, u64)> {
    let read = |name| {
        std::fs::read_to_string(format!("/sys/class/net/{iface}/statistics/{name}"))
            .ok()?
            .trim()
            .parse()
            .ok()
    };
    Some((read("rx_bytes")?, read("tx_bytes")?))
}
