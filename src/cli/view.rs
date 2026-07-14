use clap::{Args, Subcommand};

#[derive(Args)]
pub struct ViewCommand {
    #[command(subcommand)]
    pub command: ViewSubcommand,
}

#[derive(Subcommand)]
pub enum ViewSubcommand {
    /// Get a database view
    Get { id: String },
    /// List views for a database
    List { database_id: String },
}
