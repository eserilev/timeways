//! The CPU, the memory, and the GPU of the processes of a model, once a second while a bench
//! runs. The CPU and the memory come from `/proc`. The GPU comes from `nvidia-smi` when it
//! runs, and else from the DRM `fdinfo` of Linux, which the Intel and AMD drivers fill.
//! Another system gets no numbers.

use serde::Serialize;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The ticks of `/proc/<pid>/stat` in one second, on every Linux that Timeways runs on.
const TICKS_PER_SECOND: f64 = 100.0;
const EVERY: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuSource {
    NvidiaSmi,
    DrmFdinfo,
    None,
}

/// The use of one second, summed over the processes of the model.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Sample {
    /// Seconds since the Unix epoch.
    pub at: u64,
    /// 100 is one full core.
    pub cpu_percent: f64,
    pub memory_mb: f64,
    /// The share of the second that the GPU worked for the model. For `nvidia-smi`, the
    /// whole GPU.
    pub gpu_percent: Option<f64>,
    pub gpu_memory_mb: Option<f64>,
}

/// Counters that only grow, and the memory, at one moment.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Counters {
    cpu_ticks: u64,
    rss_kb: u64,
    gpu_ns: Option<u64>,
    gpu_kib: Option<u64>,
    /// `nvidia-smi` gives the busy share itself.
    gpu_percent: Option<f64>,
}

/// One client of a GPU in the `fdinfo` of a process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrmClient {
    pub id: u64,
    pub engine_ns: u64,
    pub resident_kib: u64,
}

/// Samples once a second on a thread of its own, until `stop`.
pub struct Sampler {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<Vec<Sample>>,
    pub source: GpuSource,
}

impl Sampler {
    /// Samples the processes whose name holds one of `names`.
    #[must_use]
    pub fn start(names: Vec<String>) -> Sampler {
        let source = gpu_source();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = std::thread::spawn(move || sample_until(&flag, &names, source));
        Sampler {
            stop,
            thread,
            source,
        }
    }

    /// Every sample, oldest first.
    #[must_use]
    pub fn stop(self) -> Vec<Sample> {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.join().unwrap_or_default()
    }
}

fn gpu_source() -> GpuSource {
    let nvidia = Command::new("nvidia-smi")
        .arg("-L")
        .output()
        .is_ok_and(|output| output.status.success());
    if nvidia {
        return GpuSource::NvidiaSmi;
    }
    if std::path::Path::new("/proc/self/fdinfo").exists() {
        return GpuSource::DrmFdinfo;
    }
    GpuSource::None
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

fn sample_until(stop: &AtomicBool, names: &[String], source: GpuSource) -> Vec<Sample> {
    let mut samples = Vec::new();
    let mut before = (Instant::now(), counters(names, source));
    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(EVERY);
        let now = (Instant::now(), counters(names, source));
        samples.push(sample_between(&before, &now));
        before = now;
    }
    samples
}

#[allow(
    clippy::cast_precision_loss,
    reason = "counters of one second are small"
)]
fn sample_between(before: &(Instant, Counters), now: &(Instant, Counters)) -> Sample {
    let seconds = now
        .0
        .duration_since(before.0)
        .as_secs_f64()
        .max(f64::EPSILON);
    let cpu = now.1.cpu_ticks.saturating_sub(before.1.cpu_ticks) as f64;
    let gpu_busy = match (before.1.gpu_ns, now.1.gpu_ns) {
        (Some(old), Some(new)) => Some(new.saturating_sub(old) as f64 / 1e9 / seconds * 100.0),
        _ => now.1.gpu_percent,
    };
    Sample {
        at: unix_now(),
        cpu_percent: cpu / TICKS_PER_SECOND / seconds * 100.0,
        memory_mb: now.1.rss_kb as f64 / 1024.0,
        gpu_percent: gpu_busy,
        gpu_memory_mb: now.1.gpu_kib.map(|kib| kib as f64 / 1024.0),
    }
}

fn counters(names: &[String], source: GpuSource) -> Counters {
    let pids = pids_named(names);
    let mut counters = Counters::default();
    for pid in &pids {
        let read = |file: &str| std::fs::read_to_string(format!("/proc/{pid}/{file}"));
        counters.cpu_ticks += read("stat")
            .ok()
            .and_then(|stat| cpu_ticks(&stat))
            .unwrap_or(0);
        counters.rss_kb += read("status")
            .ok()
            .and_then(|status| rss_kb(&status))
            .unwrap_or(0);
    }
    match source {
        GpuSource::DrmFdinfo => {
            let clients = drm_clients(&pids);
            counters.gpu_ns = Some(clients.iter().map(|client| client.engine_ns).sum());
            counters.gpu_kib = Some(clients.iter().map(|client| client.resident_kib).sum());
        }
        GpuSource::NvidiaSmi => {
            counters.gpu_percent = nvidia_busy();
            counters.gpu_kib = nvidia_memory_mib(&pids).map(|mib| mib * 1024);
        }
        GpuSource::None => {}
    }
    counters
}

fn pids_named(names: &[String]) -> Vec<u32> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok()?.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| {
            let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
            names.iter().any(|name| comm.trim().contains(name.as_str()))
        })
        .collect()
}

/// The user and system ticks of `/proc/<pid>/stat`. The name of the program can hold
/// spaces and parentheses, so the fields count from its last `)`.
#[must_use]
pub fn cpu_ticks(stat: &str) -> Option<u64> {
    let after_name = &stat[stat.rfind(')')? + 1..];
    let fields: Vec<&str> = after_name.split_whitespace().collect();
    // Fields 14 and 15 of the file are the 12th and 13th after the name.
    let user: u64 = fields.get(11)?.parse().ok()?;
    let system: u64 = fields.get(12)?.parse().ok()?;
    Some(user + system)
}

/// `VmRSS` of `/proc/<pid>/status`, in kB.
#[must_use]
pub fn rss_kb(status: &str) -> Option<u64> {
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}

/// The GPU client of one `fdinfo` file: its busy time on every engine, and its memory.
#[must_use]
pub fn drm_client(fdinfo: &str) -> Option<DrmClient> {
    let mut id = None;
    let mut client = DrmClient {
        id: 0,
        engine_ns: 0,
        resident_kib: 0,
    };
    for line in fdinfo.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let mut words = value.split_whitespace();
        let number: u64 = words.next().and_then(|word| word.parse().ok()).unwrap_or(0);
        let unit = words.next().unwrap_or("");
        if key == "drm-client-id" {
            id = Some(number);
        } else if key.starts_with("drm-engine-") && !key.starts_with("drm-engine-capacity") {
            client.engine_ns += number;
        } else if key.starts_with("drm-resident-") || key.starts_with("drm-memory-") {
            client.resident_kib += match unit {
                "MiB" => number * 1024,
                "GiB" => number * 1024 * 1024,
                "KiB" => number,
                _ => number / 1024,
            };
        }
    }
    client.id = id?;
    Some(client)
}

/// Each client once, though several files of a process can share it.
fn drm_clients(pids: &[u32]) -> Vec<DrmClient> {
    let mut clients: Vec<DrmClient> = Vec::new();
    for pid in pids {
        let Ok(files) = std::fs::read_dir(format!("/proc/{pid}/fdinfo")) else {
            continue;
        };
        for file in files.flatten() {
            let text = std::fs::read_to_string(file.path()).unwrap_or_default();
            let Some(client) = drm_client(&text) else {
                continue;
            };
            if !clients.iter().any(|known| known.id == client.id) {
                clients.push(client);
            }
        }
    }
    clients
}

fn nvidia_query(args: &[&str]) -> Option<String> {
    let output = Command::new("nvidia-smi").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).to_string())
}

fn nvidia_busy() -> Option<f64> {
    let text = nvidia_query(&[
        "--query-gpu=utilization.gpu",
        "--format=csv,noheader,nounits",
    ])?;
    text.lines().next()?.trim().parse().ok()
}

fn nvidia_memory_mib(pids: &[u32]) -> Option<u64> {
    let text = nvidia_query(&[
        "--query-compute-apps=pid,used_memory",
        "--format=csv,noheader,nounits",
    ])?;
    Some(nvidia_memory_of(&text, pids))
}

/// The MiB of the processes `pids` in the CSV of `nvidia-smi --query-compute-apps`.
#[must_use]
pub fn nvidia_memory_of(csv: &str, pids: &[u32]) -> u64 {
    csv.lines()
        .filter_map(|line| {
            let (pid, mib) = line.split_once(',')?;
            let pid: u32 = pid.trim().parse().ok()?;
            let mib: u64 = mib.trim().parse().ok()?;
            pids.contains(&pid).then_some(mib)
        })
        .sum()
}
