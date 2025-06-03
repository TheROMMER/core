use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub(crate) command: Option<Commands>,

    #[arg(short, long, default_value = "ROMMER.yaml")]
    pub config: String,

    #[arg(short, long, default_value = ".download")]
    pub romzip: String,

    #[arg(short, long, help = "Override cleanup setting from config")]
    pub no_cleanup: bool,

    #[arg(short, long, help = "Skip signing the final ROM")]
    pub skip_signing: bool,

    #[arg(short, long, help = "Running in dry-run mode")]
    pub dry_run: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new patch structure
    Init {
        /// Optional name for the patch folder
        #[arg(short, long, help = "Optional name for the patch folder", default_value = "my-rom")]
        name: Option<String>,
    },
}