use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Measurement {
    pub at: u64,
    pub download: Option<f64>,
    pub upload: Option<f64>,
    pub latency: Option<f64>,
    pub healthy: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    pub uuid: String,
    pub name: String,
    pub ssid: String,
    pub signal: u8,
    pub band: String,
    pub active: bool,
    pub measurement: Option<Measurement>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub at: u64,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub enabled: bool,
    pub updated_at: u64,
    pub interface: String,
    pub current: String,
    pub phase: String,
    pub message: String,
    pub networks: Vec<Network>,
    pub download_rate: f64,
    pub upload_rate: f64,
    pub events: Vec<Event>,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            enabled: true,
            updated_at: now(),
            interface: String::new(),
            current: String::new(),
            phase: "starting".into(),
            message: "Looking for Wi-Fi".into(),
            networks: vec![],
            download_rate: 0.,
            upload_rate: 0.,
            events: vec![],
        }
    }
}
pub fn score(m: &Measurement, at: u64) -> Option<f64> {
    if !m.healthy || at.saturating_sub(m.at) > 1800 {
        return None;
    }
    let down = m.download?;
    let latency = m.latency?;
    Some((down * 0.7 + m.upload? * 0.3) / (1. + latency / 150.))
}
pub fn better(candidate: &Network, current: &Network, at: u64) -> bool {
    match (
        candidate.measurement.as_ref().and_then(|m| score(m, at)),
        current.measurement.as_ref().and_then(|m| score(m, at)),
    ) {
        (Some(a), Some(b)) => a > b * 1.25 && a > b + 1.,
        _ => false,
    }
}
// nmcli escapes delimiters and backslashes in terse output.
pub fn fields(line: &str) -> Vec<String> {
    let mut out = vec![];
    let mut s = String::new();
    let mut escape = false;
    for c in line.chars() {
        if escape {
            s.push(c);
            escape = false;
        } else if c == '\\' {
            escape = true;
        } else if c == ':' {
            out.push(s);
            s = String::new();
        } else {
            s.push(c);
        }
    }
    if escape {
        s.push('\\');
    }
    out.push(s);
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    fn net(d: f64, at: u64) -> Network {
        Network {
            measurement: Some(Measurement {
                at,
                download: Some(d),
                upload: Some(10.),
                latency: Some(15.),
                healthy: true,
            }),
            ..Default::default()
        }
    }
    #[test]
    fn no_flapping_on_small_difference() {
        assert!(!better(&net(11., 100), &net(10., 100), 101));
        assert!(better(&net(25., 100), &net(10., 100), 101));
    }
    #[test]
    fn stale_and_failed_never_win() {
        assert!(!better(&net(90., 1), &net(10., 2000), 2001));
        let mut n = net(90., 100);
        n.measurement.as_mut().unwrap().healthy = false;
        assert!(!better(&n, &net(10., 100), 101));
    }
    #[test]
    fn unknown_is_not_zero() {
        let mut n = net(90., 100);
        n.measurement.as_mut().unwrap().download = None;
        assert!(!better(&n, &net(10., 100), 101));
    }
    #[test]
    fn escaped_names() {
        assert_eq!(
            fields(r"uuid:my\:wifi:back\\slash"),
            vec!["uuid", "my:wifi", r"back\slash"]
        );
    }
}
