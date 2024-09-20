use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Create config.toml file if not exists or is broken.
    #[arg(long)]
    pub create_config: bool,
    #[arg(long)]
    pub create_databases: bool,
    #[arg(long)]
    pub create_selfsigned: bool,
    #[arg(long, requires("create_selfsigned"), default_value = "localhost")]
    pub selfsigned_domain: String,
    #[arg(long, requires("create_selfsigned"), default_value = "key.pem")]
    pub selfsigned_keyfile: String,
    #[arg(long, requires("create_selfsigned"), default_value = "cert.pem")]
    pub selfsigned_certfile: String,
}
