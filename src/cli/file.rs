use clap::{Args, Subcommand};

#[derive(Args)]
pub struct FileCommand {
    #[command(subcommand)]
    pub command: FileSubcommand,
}

#[derive(Subcommand)]
pub enum FileSubcommand {
    /// Upload a file
    Upload {
        /// Path to the file
        path: String,
        /// Parent page ID
        #[arg(long)]
        parent: String,
    },
}
