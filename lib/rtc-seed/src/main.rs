mod config;
mod seeder;
mod torrent;
mod tracker;

use clap::Parser;
use config::structs::seeder_config::SeederConfig;
use config::structs::torrents_file::TorrentsFile;
use seeder::structs::seeder::Seeder;
use std::path::{
    Path,
    PathBuf
};
use std::time::SystemTime;
use torrent::structs::torrent_builder::TorrentBuilder;
use torrent::enums::torrent_version::TorrentVersion;

#[derive(Parser, Debug)]
#[command(
    name = "rtc-seed",
    about = "Native Rust WebRTC seeder — create a .torrent and seed it over WebRTC"
)]
struct Cli {
    #[arg(long, value_name = "FILE")]
    torrents: Option<PathBuf>,
    #[arg(long, default_value = "http://127.0.0.1:6969/announce")]
    tracker: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long = "webseed")]
    webseeds: Vec<String>,
    #[arg(long = "ice")]
    ice_servers: Vec<String>,
    #[arg(long, default_value = "5000")]
    rtc_interval: u64,
    #[arg(long, default_value = "v1")]
    torrent_version: String,
    files: Vec<PathBuf>,
}

#[tokio::main]
async fn main() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("[{}] {}", record.level(), message))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stderr())
        .apply()
        .expect("failed to initialize logging");
    let cli = Cli::parse();
    if let Some(yaml_path) = cli.torrents {
        let single_mode_used = !cli.files.is_empty()
            || cli.name.is_some()
            || cli.out.is_some()
            || !cli.webseeds.is_empty()
            || !cli.ice_servers.is_empty();
        if single_mode_used {
            eprintln!(
                "Error: --torrents cannot be combined with single-torrent options \
                 (positional files, --name, --out, --webseed, --ice)."
            );
            std::process::exit(1);
        }
        if !yaml_path.exists() {
            eprintln!("Torrents file not found: {}", yaml_path.display());
            std::process::exit(1);
        }
        run_torrents_mode(yaml_path).await;
    } else {
        if cli.files.is_empty() {
            eprintln!("Error: at least one file is required (or use --torrents <yaml>).");
            std::process::exit(1);
        }
        for path in &cli.files {
            if !path.exists() {
                eprintln!("File not found: {}", path.display());
                std::process::exit(1);
            }
        }
        let ice_servers = if cli.ice_servers.is_empty() {
            vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ]
        } else {
            cli.ice_servers
        };
        let version = match cli.torrent_version.as_str() {
            "v2" => TorrentVersion::V2,
            "hybrid" => TorrentVersion::Hybrid,
            _ => TorrentVersion::V1,
        };
        let config = SeederConfig {
            tracker_url: cli.tracker,
            file_paths: cli.files,
            name: cli.name,
            out_file: cli.out,
            webseed_urls: cli.webseeds,
            ice_servers,
            rtc_interval_ms: cli.rtc_interval,
            version,
        };
        println!("=== RtcTorrent Seeder (Rust native) ===");
        println!("Tracker : {}", config.tracker_url);
        let file_list: Vec<String> = config
            .file_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        println!("Files   : {}", file_list.join(", "));
        if !config.webseed_urls.is_empty() {
            println!("Webseeds: {}", config.webseed_urls.join(", "));
        }
        println!();
        print!("Creating torrent (hashing pieces)… ");
        let torrent_info = match TorrentBuilder::build(&config) {
            Ok(ti) => {
                println!("done.");
                ti
            }
            Err(e) => {
                eprintln!("\nFailed to create torrent: {}", e);
                std::process::exit(1);
            }
        };
        let mut seeder = Seeder::new(config, torrent_info);
        if let Err(e) = seeder.run().await {
            eprintln!("Fatal: {}", e);
            std::process::exit(1);
        }
    }
}

fn load_yaml_entries(path: &Path) -> Result<Vec<(String, SeederConfig)>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let file: TorrentsFile = serde_yaml::from_str(&content)?;
    let mut result = Vec::new();
    for (i, entry) in file.torrents.iter().enumerate() {
        match entry.to_seeder_config() {
            Ok(cfg) => {
                let label = cfg
                    .name
                    .clone()
                    .or_else(|| cfg.file_paths.first().map(|p| p.display().to_string()))
                    .unwrap_or_else(|| format!("torrent-{}", i));
                result.push((label, cfg));
            }
            Err(e) => {
                eprintln!("[rtc-seed] Skipping entry {}: {}", i, e);
            }
        }
    }
    Ok(result)
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

async fn seed_one(label: String, config: SeederConfig) {
    println!("[{}] Tracker  : {}", label, config.tracker_url);
    let files: Vec<String> = config.file_paths.iter().map(|p| p.display().to_string()).collect();
    println!("[{}] Files    : {}", label, files.join(", "));
    if !config.webseed_urls.is_empty() {
        println!("[{}] Webseeds : {}", label, config.webseed_urls.join(", "));
    }
    let version_str = match config.version {
        TorrentVersion::V1 => "v1",
        TorrentVersion::V2 => "v2",
        TorrentVersion::Hybrid => "hybrid",
    };
    print!("[{}] Hashing pieces ({})… ", label, version_str);
    let torrent_info = match TorrentBuilder::build(&config) {
        Ok(ti) => {
            println!("done.");
            ti
        }
        Err(e) => {
            eprintln!("\n[{}] Failed to create torrent: {}", label, e);
            return;
        }
    };
    let mut seeder = Seeder::new(config, torrent_info);
    if let Err(e) = seeder.run().await {
        eprintln!("[{}] Fatal: {}", label, e);
    }
}

async fn run_torrents_mode(yaml_path: PathBuf) {
    println!("=== RtcTorrent Seeder (Rust native, multi-torrent mode) ===");
    println!("Config  : {}", yaml_path.display());
    println!();
    #[cfg(unix)]
    let mut sighup = {
        use tokio::signal::unix::{signal, SignalKind};
        signal(SignalKind::hangup()).expect("failed to install SIGHUP handler")
    };
    loop {
        let entries = match load_yaml_entries(&yaml_path) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("[rtc-seed] Failed to load {}: {}", yaml_path.display(), e);
                std::process::exit(1);
            }
        };
        if entries.is_empty() {
            eprintln!("[rtc-seed] No valid torrent entries found in YAML — nothing to seed.");
            std::process::exit(1);
        }
        println!("[rtc-seed] Starting {} torrent(s)…", entries.len());
        let handles: Vec<_> = entries
            .into_iter()
            .map(|(label, cfg)| tokio::spawn(seed_one(label, cfg)))
            .collect();
        let initial_mtime = file_mtime(&yaml_path);
        let should_reload = 'wait: loop {
            #[cfg(unix)]
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break 'wait false,
                _ = sighup.recv() => {
                    println!("[rtc-seed] SIGHUP received — reloading…");
                    break 'wait true;
                },
                _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                    if file_mtime(&yaml_path) != initial_mtime {
                        println!("[rtc-seed] Config file changed on disk — reloading…");
                        break 'wait true;
                    }
                }
            }
            #[cfg(not(unix))]
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break 'wait false,
                _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                    if file_mtime(&yaml_path) != initial_mtime {
                        println!("[rtc-seed] Config file changed on disk — reloading…");
                        break 'wait true;
                    }
                }
            }
        };
        for h in &handles {
            h.abort();
        }
        for h in handles {
            let _ = h.await;
        }
        if !should_reload {
            println!("[rtc-seed] Shutting down.");
            break;
        }
        println!("[rtc-seed] Applying new config…\n");
    }
}