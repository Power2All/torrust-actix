//! Command-line interface argument parsing.
//!
//! This module defines the CLI arguments for the Torrust-Actix tracker binary.
//! It uses the `clap` crate for argument parsing with derive macros.

use clap::Parser;

/// Command-line interface arguments for the Torrust-Actix tracker.
///
/// This struct defines all available command-line options for the tracker binary.
/// Options are organized into groups for configuration generation, database setup,
/// SSL certificate generation, and data import/export.
///
/// # Examples
///
/// ```bash
/// # Create a new configuration file
/// torrust-actix --create-config
///
/// # Create database tables
/// torrust-actix --create-database
///
/// # Generate self-signed SSL certificate
/// torrust-actix --create-selfsigned --selfsigned-domain example.com
///
/// # Export tracker data
/// torrust-actix --export --export-file-torrents data/torrents.json
///
/// # Import tracker data
/// torrust-actix --import --import-file-torrents data/torrents.json
/// ```
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Create a new configuration file with default values.
    ///
    /// When this flag is set, the tracker will generate a new `config.toml`
    /// file in the current directory with all available configuration options
    /// set to their default values.
    #[arg(long)]
    pub create_config: bool,

    /// Create database tables using the configured database backend.
    ///
    /// This flag initializes the database schema by creating all required
    /// tables (torrents, whitelist, blacklist, keys, users) if they don't exist.
    /// The database connection settings are read from the configuration file.
    #[arg(long)]
    pub create_database: bool,

    /// Generate a self-signed SSL certificate for development/testing.
    ///
    /// Creates a self-signed X.509 certificate and private key for use with
    /// HTTPS endpoints. This should only be used for development or testing;
    /// production deployments should use certificates from a trusted CA.
    #[arg(long)]
    pub create_selfsigned: bool,

    /// Domain name for the self-signed certificate.
    ///
    /// Specifies the Common Name (CN) and Subject Alternative Name (SAN)
    /// for the generated certificate. Defaults to "localhost".
    ///
    /// Requires: `--create-selfsigned`
    #[arg(long, requires("create_selfsigned"), default_value = "localhost")]
    pub selfsigned_domain: String,

    /// Output file path for the private key (PEM format).
    ///
    /// Specifies where to save the generated private key. Defaults to "key.pem".
    ///
    /// Requires: `--create-selfsigned`
    #[arg(long, requires("create_selfsigned"), default_value = "key.pem")]
    pub selfsigned_keyfile: String,

    /// Output file path for the certificate (PEM format).
    ///
    /// Specifies where to save the generated certificate. Defaults to "cert.pem".
    ///
    /// Requires: `--create-selfsigned`
    #[arg(long, requires("create_selfsigned"), default_value = "cert.pem")]
    pub selfsigned_certfile: String,

    /// Export tracker data to JSON files.
    ///
    /// Exports all tracker data (torrents, whitelists, blacklists, keys, users)
    /// to separate JSON files for backup or migration purposes.
    #[arg(long)]
    pub export: bool,

    /// Output file path for torrents export.
    ///
    /// Specifies where to save the exported torrent data. Defaults to "torrents.json".
    ///
    /// Requires: `--export`
    #[arg(long, requires("export"), default_value = "torrents.json")]
    pub export_file_torrents: String,

    /// Output file path for whitelists export.
    ///
    /// Specifies where to save the exported whitelist data. Defaults to "whitelists.json".
    ///
    /// Requires: `--export`
    #[arg(long, requires("export"), default_value = "whitelists.json")]
    pub export_file_whitelists: String,

    /// Output file path for blacklists export.
    ///
    /// Specifies where to save the exported blacklist data. Defaults to "blacklists.json".
    ///
    /// Requires: `--export`
    #[arg(long, requires("export"), default_value = "blacklists.json")]
    pub export_file_blacklists: String,

    /// Output file path for keys export.
    ///
    /// Specifies where to save the exported API key data. Defaults to "keys.json".
    ///
    /// Requires: `--export`
    #[arg(long, requires("export"), default_value = "keys.json")]
    pub export_file_keys: String,

    /// Output file path for users export.
    ///
    /// Specifies where to save the exported user data. Defaults to "users.json".
    ///
    /// Requires: `--export`
    #[arg(long, requires("export"), default_value = "users.json")]
    pub export_file_users: String,

    /// Import tracker data from JSON files.
    ///
    /// Imports tracker data (torrents, whitelists, blacklists, keys, users)
    /// from JSON files. Useful for restoring backups or migrating data.
    #[arg(long)]
    pub import: bool,

    /// Input file path for torrents import.
    ///
    /// Specifies the JSON file to import torrent data from. Defaults to "torrents.json".
    ///
    /// Requires: `--import`
    #[arg(long, requires("export"), default_value = "torrents.json")]
    pub import_file_torrents: String,

    /// Input file path for whitelists import.
    ///
    /// Specifies the JSON file to import whitelist data from. Defaults to "whitelists.json".
    ///
    /// Requires: `--import`
    #[arg(long, requires("export"), default_value = "whitelists.json")]
    pub import_file_whitelists: String,

    /// Input file path for blacklists import.
    ///
    /// Specifies the JSON file to import blacklist data from. Defaults to "blacklists.json".
    ///
    /// Requires: `--import`
    #[arg(long, requires("export"), default_value = "blacklists.json")]
    pub import_file_blacklists: String,

    /// Input file path for keys import.
    ///
    /// Specifies the JSON file to import API key data from. Defaults to "keys.json".
    ///
    /// Requires: `--import`
    #[arg(long, requires("export"), default_value = "keys.json")]
    pub import_file_keys: String,

    /// Input file path for users import.
    ///
    /// Specifies the JSON file to import user data from. Defaults to "users.json".
    ///
    /// Requires: `--import`
    #[arg(long, requires("export"), default_value = "users.json")]
    pub import_file_users: String,
}