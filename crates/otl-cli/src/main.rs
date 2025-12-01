//! OTL CLI - Command-line interface
//!
//! Usage:
//!   otl ingest <path>
//!   otl query <question>
//!   otl verify list
//!   otl verify approve <id>

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "otl")]
#[command(about = "Ontology-based Knowledge System CLI")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest documents into the knowledge base
    Ingest {
        /// Path to documents
        path: String,
    },
    /// Query the knowledge base
    Query {
        /// Question to ask
        question: String,
    },
    /// Verify extracted knowledge
    Verify {
        #[command(subcommand)]
        action: VerifyAction,
    },
}

#[derive(Subcommand)]
enum VerifyAction {
    /// List pending extractions
    List,
    /// Approve an extraction
    Approve { id: String },
    /// Reject an extraction
    Reject { id: String, reason: Option<String> },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest { path } => {
            println!("Ingesting documents from: {}", path);
            // TODO: Implement ingestion
        }
        Commands::Query { question } => {
            println!("Query: {}", question);
            // TODO: Implement query
        }
        Commands::Verify { action } => match action {
            VerifyAction::List => {
                println!("Listing pending extractions...");
                // TODO: Implement list
            }
            VerifyAction::Approve { id } => {
                println!("Approving extraction: {}", id);
                // TODO: Implement approve
            }
            VerifyAction::Reject { id, reason } => {
                println!("Rejecting extraction: {} (reason: {:?})", id, reason);
                // TODO: Implement reject
            }
        },
    }

    Ok(())
}
