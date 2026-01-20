use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    
    #[arg(long)]
    pub create_config: bool,
    
    #[arg(long)]
    pub create_database: bool,
    
    #[arg(long)]
    pub create_selfsigned: bool,

    
    #[arg(long, requires("create_selfsigned"), default_value = "localhost")]
    pub selfsigned_domain: String,
    
    #[arg(long, requires("create_selfsigned"), default_value = "key.pem")]
    pub selfsigned_keyfile: String,
    
    #[arg(long, requires("create_selfsigned"), default_value = "cert.pem")]
    pub selfsigned_certfile: String,

    
    #[arg(long)]
    pub export: bool,
    
    #[arg(long, requires("export"), default_value = "torrents.json")]
    pub export_file_torrents: String,
    
    #[arg(long, requires("export"), default_value = "whitelists.json")]
    pub export_file_whitelists: String,
    
    #[arg(long, requires("export"), default_value = "blacklists.json")]
    pub export_file_blacklists: String,
    
    #[arg(long, requires("export"), default_value = "keys.json")]
    pub export_file_keys: String,
    
    #[arg(long, requires("export"), default_value = "users.json")]
    pub export_file_users: String,

    
    #[arg(long)]
    pub import: bool,
    
    #[arg(long, requires("export"), default_value = "torrents.json")]
    pub import_file_torrents: String,
    
    #[arg(long, requires("export"), default_value = "whitelists.json")]
    pub import_file_whitelists: String,
    
    #[arg(long, requires("export"), default_value = "blacklists.json")]
    pub import_file_blacklists: String,
    
    #[arg(long, requires("export"), default_value = "keys.json")]
    pub import_file_keys: String,
    
    #[arg(long, requires("export"), default_value = "users.json")]
    pub import_file_users: String,
}