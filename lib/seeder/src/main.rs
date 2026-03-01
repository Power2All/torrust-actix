mod config;
mod seeder;
mod stats;
mod torrent;
mod tracker;
mod web;

use clap::{
    Parser,
    Subcommand
};
use config::enums::seed_protocol::SeedProtocol;
use config::structs::proxy_config::ProxyConfig;
use config::structs::seeder_config::SeederConfig;
use config::structs::torrents_file::TorrentsFile;
use config::structs::web_config::WebConfig;
use seeder::seeder::run_shared_listener;
use seeder::structs::seeder::Seeder;
use seeder::structs::torrent_registry::new_registry;
use stats::shared_stats::new_shared_stats;
use std::path::{
    Path,
    PathBuf
};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use torrent::enums::torrent_version::TorrentVersion;
use torrent::structs::torrent_builder::TorrentBuilder;

#[derive(Subcommand, Debug)]
enum SubCmd {
    #[command(name = "hash-password")]
    HashPassword {
        password: Option<String>,
    },
}

#[derive(Parser, Debug)]
#[command(
    name = "seeder",
    about = "Unified BT+RTC seeder — seed files over BitTorrent and/or WebRTC simultaneously"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<SubCmd>,
    #[arg(long, value_name = "FILE")]
    torrents: Option<PathBuf>,
    #[arg(long = "tracker")]
    trackers: Vec<String>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long = "webseed")]
    webseeds: Vec<String>,
    #[arg(long, default_value = "6881")]
    port: u16,
    #[arg(long, default_value = "false")]
    upnp: bool,
    #[arg(long = "ice")]
    ice_servers: Vec<String>,
    #[arg(long, default_value = "5000")]
    rtc_interval: u64,
    #[arg(long, value_name = "PROTOCOL", help = "Protocol: bt, rtc, or both (default: both)")]
    protocol: Option<String>,
    #[arg(long, default_value = "v1")]
    torrent_version: String,
    #[arg(long, value_name = "FILE")]
    torrent_file: Option<PathBuf>,
    #[arg(long, value_name = "MAGNET")]
    magnet: Option<String>,
    #[arg(long)]
    web_port: Option<u16>,
    #[arg(long)]
    web_password: Option<String>,
    #[arg(long, value_name = "FILE")]
    web_cert: Option<PathBuf>,
    #[arg(long, value_name = "FILE")]
    web_key: Option<PathBuf>,
    #[arg(long)]
    proxy_type: Option<String>,
    #[arg(long)]
    proxy_host: Option<String>,
    #[arg(long)]
    proxy_port: Option<u16>,
    #[arg(long)]
    proxy_user: Option<String>,
    #[arg(long)]
    proxy_pass: Option<String>,
    #[arg(long)]
    log_level: Option<String>,
    files: Vec<PathBuf>,
}

fn parse_protocol(s: Option<&str>) -> SeedProtocol {
    match s.map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("bt") => SeedProtocol::Bt,
        Some("rtc") => SeedProtocol::Rtc,
        _ => SeedProtocol::Both,
    }
}

fn spawn_web_server(
    web_cfg: WebConfig,
    yaml_path: PathBuf,
    shared_file: Arc<RwLock<TorrentsFile>>,
    stats: crate::stats::shared_stats::SharedStats,
    reload_tx: tokio::sync::watch::Sender<()>,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build web server runtime");
        rt.block_on(async move {
            if let Err(e) = web::server::start(web_cfg, yaml_path, shared_file, stats, reload_tx).await {
                log::error!("[Web] Server error: {}", e);
            }
        });
    });
}

fn parse_log_level(s: &str) -> log::LevelFilter {
    match s.to_ascii_lowercase().as_str() {
        "error" => log::LevelFilter::Error,
        "warn"  => log::LevelFilter::Warn,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _       => log::LevelFilter::Info,
    }
}

fn read_yaml_log_level(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let file: TorrentsFile = serde_yaml::from_str(&content).ok()?;
    file.config.log_level
}

fn build_proxy_from_cli(cli: &Cli) -> Option<ProxyConfig> {
    if let (Some(proxy_type), Some(proxy_host), Some(proxy_port)) =
        (&cli.proxy_type, &cli.proxy_host, cli.proxy_port)
    {
        Some(ProxyConfig {
            proxy_type: proxy_type.clone(),
            host: proxy_host.clone(),
            port: proxy_port,
            username: cli.proxy_user.clone(),
            password: cli.proxy_pass.clone(),
        })
    } else {
        None
    }
}

fn hash_password_cmd(password: Option<String>) {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let pw = match password {
        Some(p) => p,
        None => {
            let first = rpassword::prompt_password("Enter password: ")
                .expect("failed to read password");
            let second = rpassword::prompt_password("Confirm password: ")
                .expect("failed to read password");
            if first != second {
                eprintln!("Error: passwords do not match.");
                std::process::exit(1);
            }
            first
        }
    };
    if pw.is_empty() {
        eprintln!("Error: password must not be empty.");
        std::process::exit(1);
    }
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .expect("failed to hash password")
        .to_string();
    println!("{}", hash);
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Some(SubCmd::HashPassword { password }) = cli.command {
        hash_password_cmd(password);
        return;
    }
    let level_filter = {
        let s = cli.log_level.clone()
            .or_else(|| cli.torrents.as_deref().and_then(|p| read_yaml_log_level(Path::new(p))))
            .unwrap_or_else(|| "info".to_string());
        parse_log_level(&s)
    };
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("[{}] {}", record.level(), message))
        })
        .level(level_filter)
        .chain(std::io::stderr())
        .apply()
        .expect("failed to initialize logging");
    if let Some(yaml_path) = cli.torrents.clone() {
        let single_mode_used = !cli.files.is_empty()
            || cli.name.is_some()
            || cli.out.is_some()
            || !cli.webseeds.is_empty()
            || !cli.ice_servers.is_empty()
            || cli.torrent_file.is_some()
            || cli.magnet.is_some();
        if single_mode_used {
            eprintln!(
                "Error: --torrents cannot be combined with single-torrent options \
                 (positional files, --name, --out, --webseed, --ice, --torrent-file, --magnet)."
            );
            std::process::exit(1);
        }
        let cli_proxy = build_proxy_from_cli(&cli);
        let cli_web = WebConfig {
            port: cli.web_port.unwrap_or(0),
            password: cli.web_password.clone(),
            cert_path: cli.web_cert.clone(),
            key_path: cli.web_key.clone(),
        };
        run_torrents_mode(yaml_path, cli_proxy, cli_web, cli.upnp, cli.protocol.as_deref()).await;
    } else {
        let has_input = !cli.files.is_empty() || cli.torrent_file.is_some();
        if !has_input {
            if cli.web_port.is_some() {
                let yaml_path = PathBuf::from("torrents.yaml");
                let cli_proxy = build_proxy_from_cli(&cli);
                let cli_web = WebConfig {
                    port: cli.web_port.unwrap_or(0),
                    password: cli.web_password.clone(),
                    cert_path: cli.web_cert.clone(),
                    key_path: cli.web_key.clone(),
                };
                run_torrents_mode(yaml_path, cli_proxy, cli_web, cli.upnp, cli.protocol.as_deref()).await;
                return;
            }
            eprintln!("Error: provide file(s) to seed, or --torrent-file <path>, or --torrents <yaml>.");
            std::process::exit(1);
        }
        for path in &cli.files {
            if !path.exists() {
                eprintln!("File not found: {}", path.display());
                std::process::exit(1);
            }
        }
        if let Some(tf) = &cli.torrent_file
            && !tf.exists()
        {
            eprintln!("Torrent file not found: {}", tf.display());
            std::process::exit(1);
        }
        let version = match cli.torrent_version.as_str() {
            "v2" => TorrentVersion::V2,
            "hybrid" => TorrentVersion::Hybrid,
            _ => TorrentVersion::V1,
        };
        let proxy = build_proxy_from_cli(&cli);
        let ice_servers = if cli.ice_servers.is_empty() {
            vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ]
        } else {
            cli.ice_servers.clone()
        };
        let protocol = parse_protocol(cli.protocol.as_deref());
        let config = SeederConfig {
            tracker_urls: cli.trackers,
            file_paths: cli.files,
            name: cli.name,
            out_file: cli.out,
            webseed_urls: cli.webseeds,
            listen_port: cli.port,
            upnp: cli.upnp,
            ice_servers,
            rtc_interval_ms: cli.rtc_interval,
            protocol: protocol.clone(),
            version,
            torrent_file: cli.torrent_file,
            magnet: cli.magnet,
            upload_limit: None,
            proxy,
            show_stats: true,
        };
        println!("=== Seeder (BT+RTC) ===");
        println!("Protocol: {}", match &protocol {
            SeedProtocol::Bt => "bt (BitTorrent only)",
            SeedProtocol::Rtc => "rtc (WebRTC only)",
            SeedProtocol::Both => "both (BT + RTC)",
        });
        if config.tracker_urls.is_empty() && config.torrent_file.is_none() && config.magnet.is_none() {
            println!("Trackers: (none — seeding without announcing)");
        } else if !config.tracker_urls.is_empty() {
            println!("Trackers: {}", config.tracker_urls.join(", "));
        }
        if let Some(tf) = &config.torrent_file {
            println!("Torrent : {}", tf.display());
        }
        if let Some(mag) = &config.magnet {
            println!("Magnet  : {}…", &mag[..mag.len().min(60)]);
        }
        let file_list: Vec<String> = config
            .file_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        if !file_list.is_empty() {
            println!("Files   : {}", file_list.join(", "));
        }
        if config.protocol.has_bt() {
            println!("Port    : {}", config.listen_port);
        }
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
        if let Some(web_port) = cli.web_port {
            let yaml_path = PathBuf::from("torrents.yaml");
            let shared_file = Arc::new(RwLock::new(TorrentsFile::default()));
            let shared_stats = new_shared_stats();
            let (reload_tx, _reload_rx) = tokio::sync::watch::channel(());
            let web_cfg = WebConfig {
                port: web_port,
                password: cli.web_password.clone(),
                cert_path: cli.web_cert.clone(),
                key_path: cli.web_key.clone(),
            };
            spawn_web_server(web_cfg, yaml_path, shared_file, shared_stats, reload_tx);
        }
        let mut s = Seeder::new(config, torrent_info);
        if let Err(e) = s.run(None).await {
            eprintln!("Fatal: {}", e);
            std::process::exit(1);
        }
    }
}

fn load_yaml(path: &Path) -> Result<TorrentsFile, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let file: TorrentsFile = serde_yaml::from_str(&content)?;
    Ok(file)
}

#[allow(clippy::type_complexity)]
fn load_yaml_entries(
    path: &Path,
    proxy: Option<&ProxyConfig>,
    upnp: bool,
    cli_protocol: Option<&str>,
) -> Result<(TorrentsFile, Vec<(String, SeederConfig)>), Box<dyn std::error::Error>> {
    let file = load_yaml(path)?;
    let effective_proxy = proxy.or(file.config.proxy.as_ref());
    let effective_upnp = upnp || file.config.upnp.unwrap_or(false);
    let effective_show_stats = file.config.show_stats.unwrap_or(true);
    let effective_listen_port = file.config.listen_port.unwrap_or(6881);
    let effective_protocol = parse_protocol(
        cli_protocol.or(file.config.protocol.as_ref().map(|p| match p {
            SeedProtocol::Bt => "bt",
            SeedProtocol::Rtc => "rtc",
            SeedProtocol::Both => "both",
        }))
    );
    let effective_ice: Vec<String> = file.config.rtc_ice_servers.clone().unwrap_or_else(|| vec![
        "stun:stun.l.google.com:19302".to_string(),
        "stun:stun1.l.google.com:19302".to_string(),
    ]);
    let effective_rtc_interval_ms = file.config.rtc_interval_ms.unwrap_or(5000);
    let mut result = Vec::new();
    for (i, entry) in file.torrents.iter().enumerate() {
        if !entry.enabled {
            let label = entry.name.clone().unwrap_or_else(|| format!("torrent-{}", i));
            println!("[{}] disabled — skipping", label);
            continue;
        }
        match entry.to_seeder_config(
            effective_proxy,
            effective_listen_port,
            effective_protocol.clone(),
            &effective_ice,
            effective_rtc_interval_ms,
        ) {
            Ok(mut cfg) => {
                cfg.upnp = effective_upnp;
                cfg.show_stats = effective_show_stats;
                let label = cfg
                    .name
                    .clone()
                    .or_else(|| cfg.file_paths.first().map(|p| p.display().to_string()))
                    .or_else(|| cfg.torrent_file.as_ref().map(|p| p.display().to_string()))
                    .unwrap_or_else(|| format!("torrent-{}", i));
                result.push((label, cfg));
            }
            Err(e) => {
                eprintln!("[seeder] Skipping entry {}: {}", i, e);
            }
        }
    }
    Ok((file, result))
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

async fn seed_one(label: String, config: SeederConfig, registry: Option<seeder::structs::torrent_registry::TorrentRegistry>) {
    if config.tracker_urls.is_empty() && config.torrent_file.is_none() && config.magnet.is_none() {
        println!("[{}] Trackers: (none)", label);
    } else if !config.tracker_urls.is_empty() {
        println!("[{}] Trackers: {}", label, config.tracker_urls.join(", "));
    }
    if let Some(tf) = &config.torrent_file {
        println!("[{}] Torrent : {}", label, tf.display());
    }
    let files: Vec<String> = config.file_paths.iter().map(|p| p.display().to_string()).collect();
    if !files.is_empty() {
        println!("[{}] Files   : {}", label, files.join(", "));
    }
    if !config.webseed_urls.is_empty() {
        println!("[{}] Webseeds: {}", label, config.webseed_urls.join(", "));
    }
    let proto_str = match &config.protocol {
        SeedProtocol::Bt => "bt",
        SeedProtocol::Rtc => "rtc",
        SeedProtocol::Both => "both",
    };
    let version_str = match config.version {
        TorrentVersion::V1 => "v1",
        TorrentVersion::V2 => "v2",
        TorrentVersion::Hybrid => "hybrid",
    };
    print!("[{}] Hashing pieces ({}, protocol={})… ", label, version_str, proto_str);
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
    let mut s = Seeder::new(config, torrent_info);
    if let Err(e) = s.run(registry).await {
        eprintln!("[{}] Fatal: {}", label, e);
    }
}

async fn run_torrents_mode(
    yaml_path: PathBuf,
    cli_proxy: Option<ProxyConfig>,
    cli_web: WebConfig,
    cli_upnp: bool,
    cli_protocol: Option<&str>,
) {
    println!("=== Seeder (BT+RTC, multi-torrent mode) ===");
    println!("Config  : {}", yaml_path.display());
    println!();
    if !yaml_path.exists() {
        println!("[seeder] Creating empty config file: {}", yaml_path.display());
        let empty = TorrentsFile::default();
        let s = serde_yaml::to_string(&empty).expect("serialize empty TorrentsFile");
        std::fs::write(&yaml_path, s).expect("write empty YAML");
    }
    #[cfg(unix)]
    let mut sighup = {
        use tokio::signal::unix::{signal, SignalKind};
        signal(SignalKind::hangup()).expect("failed to install SIGHUP handler")
    };
    let shared_file: Arc<RwLock<TorrentsFile>> = Arc::new(RwLock::new(TorrentsFile::default()));
    let shared_stats = new_shared_stats();
    let (reload_tx, mut reload_rx) = tokio::sync::watch::channel(());
    let yaml_for_web = match load_yaml(&yaml_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[seeder] Failed to load {}: {}", yaml_path.display(), e);
            std::process::exit(1);
        }
    };
    let effective_web_port = if cli_web.port > 0 {
        Some(cli_web.port)
    } else {
        yaml_for_web.config.web_port
    };
    if let Some(web_port) = effective_web_port {
        let web_cfg = WebConfig {
            port: web_port,
            password: cli_web.password.or(yaml_for_web.config.web_password.clone()),
            cert_path: cli_web.cert_path.or(yaml_for_web.config.web_cert.clone()),
            key_path: cli_web.key_path.or(yaml_for_web.config.web_key.clone()),
        };
        let sf = Arc::clone(&shared_file);
        let ss = Arc::clone(&shared_stats);
        let rtx = reload_tx.clone();
        let yp = yaml_path.clone();
        spawn_web_server(web_cfg, yp, sf, ss, rtx);
    }
    loop {
        let (file, entries) = match load_yaml_entries(&yaml_path, cli_proxy.as_ref(), cli_upnp, cli_protocol) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("[seeder] Failed to load {}: {}", yaml_path.display(), e);
                std::process::exit(1);
            }
        };
        let effective_listen_port = file.config.listen_port.unwrap_or(6881);
        let effective_upnp = cli_upnp || file.config.upnp.unwrap_or(false);
        let effective_protocol = parse_protocol(
            cli_protocol.or(file.config.protocol.as_ref().map(|p| match p {
                SeedProtocol::Bt => "bt",
                SeedProtocol::Rtc => "rtc",
                SeedProtocol::Both => "both",
            }))
        );
        {
            let mut sf = shared_file.write().await;
            *sf = file;
        }
        if entries.is_empty() {
            println!("[seeder] No enabled torrent entries — waiting for changes…");
        } else {
            println!("[seeder] Starting {} torrent(s)…", entries.len());
        }
        let registry = new_registry();
        let listener_handle = if effective_protocol.has_bt() {
            let reg = Arc::clone(&registry);
            Some(tokio::spawn(async move {
                run_shared_listener(effective_listen_port, reg, effective_upnp).await;
            }))
        } else {
            None
        };
        let handles: Vec<_> = entries
            .into_iter()
            .map(|(label, cfg)| {
                let reg = if cfg.protocol.has_bt() { Some(Arc::clone(&registry)) } else { None };
                tokio::spawn(seed_one(label, cfg, reg))
            })
            .collect();
        let initial_mtime = file_mtime(&yaml_path);
        let should_reload = 'wait: loop {
            #[cfg(unix)]
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break 'wait false,
                _ = sighup.recv() => {
                    println!("[seeder] SIGHUP received — reloading…");
                    break 'wait true;
                },
                _ = reload_rx.changed() => {
                    println!("[seeder] Web UI triggered reload…");
                    break 'wait true;
                },
                _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                    if file_mtime(&yaml_path) != initial_mtime {
                        println!("[seeder] Config file changed on disk — reloading…");
                        break 'wait true;
                    }
                }
            }
            #[cfg(not(unix))]
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break 'wait false,
                _ = reload_rx.changed() => {
                    println!("[seeder] Web UI triggered reload…");
                    break 'wait true;
                },
                _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                    if file_mtime(&yaml_path) != initial_mtime {
                        println!("[seeder] Config file changed on disk — reloading…");
                        break 'wait true;
                    }
                }
            }
        };
        if let Some(h) = listener_handle {
            h.abort();
            let _ = h.await;
        }
        if should_reload {
            for h in &handles {
                h.abort();
            }
            for h in handles {
                let _ = h.await;
            }
            println!("[seeder] Applying new config…\n");
        } else {
            println!("[seeder] Shutting down — waiting for tracker announcements (up to 10s)…");
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                async {
                    for h in handles {
                        let _ = h.await;
                    }
                },
            ).await;
            println!("[seeder] Shutting down.");
            break;
        }
    }
}