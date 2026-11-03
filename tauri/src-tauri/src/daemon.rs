use crate::{model::*, network};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
type Shared = Arc<Mutex<Snapshot>>;
#[derive(Serialize, Deserialize, Default)]
struct Saved {
    enabled: bool,
    #[serde(default)]
    networks: Vec<Network>,
}
pub fn directory() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!(
                "internet-swap-{}",
                std::env::var("USER").unwrap_or_default()
            ))
        })
        .join("internet-swap")
}
fn settings() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join(".config"))
        .join("internet-swap/settings.json")
}
fn persist(s: &Snapshot) -> Result<(), String> {
    let path = settings();
    let dir = path.parent().unwrap();
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    let tmp = dir.join("settings.tmp");
    let mut f = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode_private()
        .open(&tmp)
        .map_err(|e| e.to_string())?;
    f.write_all(
        &serde_json::to_vec(&Saved {
            enabled: s.enabled,
            networks: s.networks.clone(),
        })
        .unwrap(),
    )
    .map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())?;
    fs::rename(tmp, path).map_err(|e| e.to_string())
}
trait Private {
    fn mode_private(&mut self) -> &mut Self;
}
impl Private for OpenOptions {
    fn mode_private(&mut self) -> &mut Self {
        use std::os::unix::fs::OpenOptionsExt;
        self.mode(0o600)
    }
}
pub fn request(command: &str) -> Result<serde_json::Value, String> {
    let mut stream = UnixStream::connect(directory().join("socket"))
        .map_err(|_| "Background service is unavailable".to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    stream
        .write_all(format!("{command}\n").as_bytes())
        .map_err(|e| e.to_string())?;
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&line).map_err(|e| e.to_string())
}
fn note(s: &Shared, phase: &str, text: &str) {
    let mut v = s.lock().unwrap();
    v.phase = phase.into();
    v.message = text.into();
    v.updated_at = now();
    if v.events.first().is_none_or(|e| e.text != text) {
        v.events.insert(
            0,
            Event {
                at: now(),
                text: text.into(),
            },
        );
        v.events.truncate(30);
    }
}
fn enabled(s: &Shared) -> bool {
    s.lock().unwrap().enabled
}
fn snapshot(s: &Shared) -> Snapshot {
    s.lock().unwrap().clone()
}
fn refresh(s: &Shared, iface: &str) -> Result<(), String> {
    let old = snapshot(s).networks;
    let nets = network::scan(iface, &old)?;
    let current = network::current(iface)?;
    let mut v = s.lock().unwrap();
    v.networks = nets;
    v.current = current;
    v.updated_at = now();
    Ok(())
}
fn record(s: &Shared, uuid: &str, m: Measurement) {
    let mut v = s.lock().unwrap();
    if let Some(n) = v.networks.iter_mut().find(|n| n.uuid == uuid) {
        n.measurement = Some(m);
    }
    let _ = persist(&v);
}
fn restore(s: &Shared, iface: &str, original: &str) {
    let result = if original.is_empty() {
        network::disconnect(iface)
    } else {
        network::activate(original, iface)
    };
    if let Err(e) = result {
        note(
            s,
            "error",
            &format!("Could not restore the previous Wi-Fi: {e}"),
        );
    } else {
        let _ = refresh(s, iface);
        note(s, "watching", "Returned to the previous Wi-Fi");
    }
}
fn test_all(s: &Shared, iface: &str) {
    let original = network::current(iface).unwrap_or_default();
    let nets = snapshot(s).networks;
    for n in nets {
        if !enabled(s) {
            break;
        }
        note(
            s,
            "testing",
            &format!("Testing {}. Wi-Fi may briefly disconnect", n.name),
        );
        if network::activate(&n.uuid, iface).is_err() {
            note(s, "error", &format!("Could not connect to {}", n.name));
            continue;
        }
        record(s, &n.uuid, network::measure(iface));
    }
    restore(s, iface, &original);
}
fn switch(s: &Shared, iface: &str, uuid: &str) -> bool {
    if !enabled(s) {
        return false;
    }
    let original = network::current(iface).unwrap_or_default();
    if original == uuid {
        return false;
    }
    note(s, "switching", "Checking another saved Wi-Fi");
    let success = network::activate(uuid, iface).is_ok() && network::health(iface).is_some();
    if success && enabled(s) {
        let _ = refresh(s, iface);
        record(s, uuid, network::measure(iface));
        note(s, "watching", "Switched to a working Wi-Fi");
        true
    } else {
        restore(s, iface, &original);
        false
    }
}
pub fn serve() -> Result<(), String> {
    let dir = directory();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // Refuse a directory owned by another user or symlink before touching its socket.
    let meta = fs::symlink_metadata(&dir).map_err(|e| e.to_string())?;
    use std::os::unix::fs::MetadataExt;
    let uid = fs::metadata("/proc/self").map_err(|e| e.to_string())?.uid();
    if !meta.is_dir() || meta.uid() != uid {
        return Err("Unsafe runtime directory".into());
    }
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode_private()
        .open(dir.join("daemon.lock"))
        .map_err(|e| e.to_string())?;
    if lock.try_lock_exclusive().is_err() {
        return Ok(());
    }
    let socket = dir.join("socket");
    let _ = fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket).map_err(|e| e.to_string())?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    let saved = match fs::read(settings()) {
        Ok(bytes) => Some(serde_json::from_slice::<Saved>(&bytes).map_err(|_| {
            "Settings are invalid. Restore or remove settings.json before restarting."
        })?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
    };
    let mut initial = Snapshot::default();
    if let Some(saved) = saved {
        initial.enabled = saved.enabled;
        initial.networks = saved.networks;
    }
    let shared = Arc::new(Mutex::new(initial));
    let service = shared.clone();
    let (tx, rx) = mpsc::sync_channel::<String>(1);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
            let mut line = String::new();
            let read = BufReader::new(&stream).take_limit_line(&mut line);
            if read.is_err() {
                continue;
            }
            let command = line.trim();
            let response = match command {
                "status" => serde_json::to_value(snapshot(&service)).unwrap(),
                "enable" | "disable" => {
                    let mut s = service.lock().unwrap();
                    let previous = s.enabled;
                    s.enabled = command == "enable";
                    match persist(&s) {
                        Ok(()) => {
                            s.phase = if s.enabled { "watching" } else { "paused" }.into();
                            s.message = if s.enabled {
                                "Automatic switching is on"
                            } else {
                                "Automatic switching is off"
                            }
                            .into();
                            serde_json::json!({"ok":true})
                        }
                        Err(e) => {
                            s.enabled = previous;
                            serde_json::json!({"error":e})
                        }
                    }
                }
                "test" => {
                    if !enabled(&service) {
                        serde_json::json!({"error":"Turn on automatic switching before testing"})
                    } else {
                        match tx.try_send(command.into()) {
                            Ok(()) => serde_json::json!({"ok":true}),
                            Err(_) => serde_json::json!({"error":"An action is already queued"}),
                        }
                    }
                }
                _ if command.starts_with("connect ") => {
                    let id = &command[8..];
                    if !snapshot(&service).networks.iter().any(|n| n.uuid == id) {
                        serde_json::json!({"error":"Network is not a saved, visible secured Wi-Fi"})
                    } else if !enabled(&service) {
                        serde_json::json!({"error":"Turn on automatic switching first"})
                    } else {
                        match tx.try_send(command.into()) {
                            Ok(()) => serde_json::json!({"ok":true}),
                            Err(_) => serde_json::json!({"error":"An action is already queued"}),
                        }
                    }
                }
                _ => serde_json::json!({"error":"Unknown command"}),
            };
            let _ = writeln!(stream, "{response}");
        }
    });
    let mut last_scan = Instant::now() - Duration::from_secs(60);
    let mut last_probe = last_scan;
    let mut last_measure = last_scan - Duration::from_secs(120);
    let mut last_switch = last_scan - Duration::from_secs(120);
    let mut failures = 0;
    let mut baseline: Option<(String, u64, u64, Instant)> = None;
    let mut streak = String::new();
    let mut wins = 0;
    let mut cooldown: std::collections::HashMap<String, u64> = Default::default();
    loop {
        let iface = match network::interface() {
            Ok(i) => i,
            Err(e) => {
                note(&shared, "unavailable", &e);
                thread::sleep(Duration::from_secs(3));
                continue;
            }
        };
        shared.lock().unwrap().interface = iface.clone();
        if let Some((rx_bytes, tx_bytes)) = network::counters(&iface) {
            let mut v = shared.lock().unwrap();
            v.download_rate = 0.;
            v.upload_rate = 0.;
            if let Some((old_iface, rx, tx, at)) = baseline.as_ref() {
                if old_iface == &iface {
                    let seconds = at.elapsed().as_secs_f64();
                    v.download_rate = rx_bytes.saturating_sub(*rx) as f64 / seconds;
                    v.upload_rate = tx_bytes.saturating_sub(*tx) as f64 / seconds;
                }
            }
            baseline = Some((iface.clone(), rx_bytes, tx_bytes, Instant::now()));
        }
        if last_scan.elapsed() >= Duration::from_secs(30) {
            match refresh(&shared, &iface) {
                Ok(()) => {}
                Err(e) => note(&shared, "error", &e),
            }
            last_scan = Instant::now();
        }
        if !enabled(&shared) {
            failures = 0;
            streak.clear();
            wins = 0;
            note(&shared, "paused", "Automatic switching is off");
            while rx.try_recv().is_ok() {}
            thread::sleep(Duration::from_secs(1));
            continue;
        }
        if let Ok(command) = rx.try_recv() {
            if command == "test" {
                test_all(&shared, &iface);
            } else if let Some(id) = command.strip_prefix("connect ") {
                switch(&shared, &iface, id);
            }
            last_switch = Instant::now();
            last_scan = Instant::now() - Duration::from_secs(60);
            last_measure = Instant::now();
        }
        if last_probe.elapsed() >= Duration::from_secs(5) {
            let actual = network::current(&iface).unwrap_or_default();
            {
                let mut v = shared.lock().unwrap();
                v.current = actual.clone();
                for n in &mut v.networks {
                    n.active = n.uuid == actual;
                }
            }
            let latency = if actual.is_empty() {
                None
            } else {
                network::health(&iface)
            };
            if let Some(latency) = latency {
                failures = 0;
                let mut v = shared.lock().unwrap();
                if let Some(m) = v
                    .networks
                    .iter_mut()
                    .find(|n| n.uuid == actual)
                    .and_then(|n| n.measurement.as_mut())
                {
                    m.latency = Some(latency);
                    m.healthy = true;
                }
            } else {
                failures += 1;
            }
            if failures >= 2 {
                note(
                    &shared,
                    "recovering",
                    "Internet checks failed. Looking for another saved Wi-Fi",
                );
                let mut candidates = snapshot(&shared).networks;
                candidates.retain(|n| {
                    n.uuid != actual
                        && cooldown
                            .get(&n.uuid)
                            .is_none_or(|at| now().saturating_sub(*at) >= 120)
                });
                candidates.sort_by(|a, b| {
                    let rank = |n: &Network| {
                        n.measurement
                            .as_ref()
                            .and_then(|m| score(m, now()))
                            .unwrap_or(-1.)
                    };
                    rank(b).total_cmp(&rank(a)).then(b.signal.cmp(&a.signal))
                });
                for n in candidates {
                    if !enabled(&shared) {
                        break;
                    }
                    if switch(&shared, &iface, &n.uuid) {
                        failures = 0;
                        last_switch = Instant::now();
                        last_measure = Instant::now();
                        break;
                    }
                    cooldown.insert(n.uuid, now());
                }
            } else if latency.is_some() {
                note(&shared, "watching", "Watching your Wi-Fi");
            }
            if latency.is_some() && last_measure.elapsed() >= Duration::from_secs(120) {
                note(&shared, "measuring", "Measuring this connection");
                record(&shared, &actual, network::measure(&iface));
                last_measure = Instant::now();
            }
            let s = snapshot(&shared);
            if failures == 0 && last_switch.elapsed() >= Duration::from_secs(120) {
                if let Some(current) = s.networks.iter().find(|n| n.uuid == actual) {
                    let candidate = s
                        .networks
                        .iter()
                        .filter(|n| {
                            n.uuid != actual
                                && cooldown
                                    .get(&n.uuid)
                                    .is_none_or(|at| now().saturating_sub(*at) >= 120)
                        })
                        .filter(|n| better(n, current, now()))
                        .max_by(|a, b| {
                            score(a.measurement.as_ref().unwrap(), now())
                                .unwrap()
                                .total_cmp(&score(b.measurement.as_ref().unwrap(), now()).unwrap())
                        });
                    if let Some(n) = candidate {
                        if streak == n.uuid {
                            wins += 1;
                        } else {
                            streak = n.uuid.clone();
                            wins = 1;
                        }
                        if wins >= 3 {
                            if !switch(&shared, &iface, &n.uuid) {
                                cooldown.insert(n.uuid.clone(), now());
                            }
                            last_switch = Instant::now();
                            last_measure = Instant::now();
                            wins = 0;
                        }
                    } else {
                        streak.clear();
                        wins = 0;
                    }
                }
            }
            last_probe = Instant::now();
        }
        shared.lock().unwrap().updated_at = now();
        thread::sleep(Duration::from_secs(1));
    }
}
trait LimitedLine {
    fn take_limit_line(&mut self, out: &mut String) -> std::io::Result<usize>;
}
impl<T: std::io::Read> LimitedLine for BufReader<T> {
    fn take_limit_line(&mut self, out: &mut String) -> std::io::Result<usize> {
        use std::io::Read;
        self.take(1024).read_line(out)
    }
}
