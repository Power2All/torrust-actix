use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Create config.toml file if not exists or is broken.
    #[arg(long)]
    pub create_config: bool,
    #[arg(long)]
    pub create_databases: bool
}
