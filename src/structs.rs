use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Create config.toml file if not exists or is broken
    #[arg(long)]
    pub create_config: bool,
    /// Create the database for the engine that is used in the config.toml
    #[arg(long)]
    pub create_database: bool,
    /// Create a development self-signed key and certificate file in PEM format
    #[arg(long)]
    pub create_selfsigned: bool,

    /// Add an extra domain/subdomain into the certificate, for development
    #[arg(long, requires("create_selfsigned"), default_value = "localhost")]
    pub selfsigned_domain: String,
    /// Give the filename of the key file of the certificate, default key.pem
    #[arg(long, requires("create_selfsigned"), default_value = "key.pem")]
    pub selfsigned_keyfile: String,
    /// Give the filename of the certificate file, default cert.pem
    #[arg(long, requires("create_selfsigned"), default_value = "cert.pem")]
    pub selfsigned_certfile: String,

    /// Create export files of the data from the database, useful for migration or backup
    #[arg(long)]
    pub export: bool,
    /// Give the filename of the JSON file for torrents, default torrents.json
    #[arg(long, requires("export"), default_value = "torrents.json")]
    pub export_file_torrents: String,
    /// Give the filename of the JSON file for whitelists, default whitelists.json
    #[arg(long, requires("export"), default_value = "whitelists.json")]
    pub export_file_whitelists: String,
    /// Give the filename of the JSON file for blacklists, default blacklists.json
    #[arg(long, requires("export"), default_value = "blacklists.json")]
    pub export_file_blacklists: String,
    /// Give the filename of the JSON file for keys, default keys.json
    #[arg(long, requires("export"), default_value = "keys.json")]
    pub export_file_keys: String,
    /// Give the filename of the JSON file for users, default users.json
    #[arg(long, requires("export"), default_value = "users.json")]
    pub export_file_users: String,

    /// Import data from JSON files
    #[arg(long)]
    pub import: bool,
    /// Give the filename of the JSON file for torrents, default torrents.json
    #[arg(long, requires("export"), default_value = "torrents.json")]
    pub import_file_torrents: String,
    /// Give the filename of the JSON file for whitelists, default whitelists.json
    #[arg(long, requires("export"), default_value = "whitelists.json")]
    pub import_file_whitelists: String,
    /// Give the filename of the JSON file for blacklists, default blacklists.json
    #[arg(long, requires("export"), default_value = "blacklists.json")]
    pub import_file_blacklists: String,
    /// Give the filename of the JSON file for keys, default keys.json
    #[arg(long, requires("export"), default_value = "keys.json")]
    pub import_file_keys: String,
    /// Give the filename of the JSON file for users, default users.json
    #[arg(long, requires("export"), default_value = "users.json")]
    pub import_file_users: String,
}
