mod config;
mod seeder;
mod torrent;
mod tracker;

use clap::Parser;
use config::structs::seeder_config::SeederConfig;
use seeder::structs::seeder::Seeder;
use std::path::PathBuf;
use torrent::structs::torrent_builder::TorrentBuilder;

#[derive(Parser, Debug)]
#[command(
    name = "rtc-seed",
    about = "Native Rust WebRTC seeder — create a .torrent and seed it over WebRTC"
)]
struct Cli {
    /// Tracker announce URL
    #[arg(long, default_value = "http://127.0.0.1:6969/announce")]
    tracker: String,

    /// Torrent name (default: first file's name)
    #[arg(long)]
    name: Option<String>,

    /// Output path for the .torrent file
    #[arg(long)]
    out: Option<PathBuf>,

    /// WebSeed URL (BEP-19, can be repeated)
    #[arg(long = "webseed")]
    webseeds: Vec<String>,

    /// ICE server URL (can be repeated; default: stun.l.google.com:19302)
    #[arg(long = "ice")]
    ice_servers: Vec<String>,

    /// RTC announce poll interval in milliseconds
    #[arg(long, default_value = "5000")]
    rtc_interval: u64,

    /// Files to seed (at least one required)
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

#[tokio::main]
async fn main() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] {}",
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stderr())
        .apply()
        .expect("failed to initialize logging");
    let cli = Cli::parse();
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
    let config = SeederConfig {
        tracker_url: cli.tracker,
        file_paths: cli.files,
        name: cli.name,
        out_file: cli.out,
        webseed_urls: cli.webseeds,
        ice_servers,
        rtc_interval_ms: cli.rtc_interval,
    };
    println!("=== RtcTorrent Seeder (Rust native) ===");
    println!("Tracker : {}", config.tracker_url);
    let file_list: Vec<String> = config.file_paths.iter().map(|p| p.display().to_string()).collect();
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