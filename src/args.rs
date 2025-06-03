use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
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