//! OTL CLI - Command-line interface
//!
//! Usage:
//! ```text
//!   otl ingest <path>
//!   otl query <question>
//!   otl verify list
//!   otl verify approve <id>
//!   otl verify reject <id> [reason]
//!   otl verify stats
//!   otl extract <path>
//! ```
//!
//! Author: hephaex@gmail.com

use std::io::{self, Write};
use std::sync::Mutex;

use clap::{Parser, Subcommand};
use futures::StreamExt;
use once_cell::sync::Lazy;
use uuid::Uuid;

use otl_core::LlmClient;
use otl_extractor::hitl::VerificationQueue;
use otl_extractor::ner::RuleBasedNer;
use otl_extractor::relation::RuleBasedRe;
use otl_extractor::{EntityExtractor, RelationExtractor};
use otl_rag::OllamaClient;

// Global verification queue (in production, this would be backed by a database)
static VERIFICATION_QUEUE: Lazy<Mutex<VerificationQueue>> =
    Lazy::new(|| Mutex::new(VerificationQueue::new().with_threshold(0.95)));

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
    /// Query the knowledge base using RAG
    Query {
        /// Question to ask
        question: String,
        /// Stream output
        #[arg(short, long)]
        stream: bool,
        /// Use Ollama (default: OpenAI if API key set)
        #[arg(long)]
        ollama: bool,
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Verify extracted knowledge (HITL)
    Verify {
        #[command(subcommand)]
        action: VerifyAction,
    },
    /// Extract entities and relations from text
    Extract {
        /// Text to extract from (or file path)
        input: String,
        /// Show entities only
        #[arg(long)]
        entities_only: bool,
        /// Show relations only
        #[arg(long)]
        relations_only: bool,
    },
}

#[derive(Subcommand)]
enum VerifyAction {
    /// List pending extractions
    List {
        /// Filter by type (entity/relation)
        #[arg(short = 't', long)]
        item_type: Option<String>,
        /// Show only first N items
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Show single item details
    Show {
        /// Item ID
        id: String,
    },
    /// Approve an extraction
    Approve {
        /// Item ID
        id: String,
        /// Optional note
        #[arg(short, long)]
        note: Option<String>,
    },
    /// Reject an extraction
    Reject {
        /// Item ID
        id: String,
        /// Rejection reason
        #[arg(short, long)]
        reason: Option<String>,
    },
    /// Show verification statistics
    Stats,
    /// Load demo data for testing
    Demo,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest { path } => {
            println!("Ingesting documents from: {path}");
            // TODO: Implement ingestion
        }
        Commands::Query {
            question,
            stream,
            ollama,
            model,
        } => {
            cmd_query(&question, stream, ollama, model.as_deref()).await?;
        }
        Commands::Extract {
            input,
            entities_only,
            relations_only,
        } => {
            cmd_extract(&input, entities_only, relations_only)?;
        }
        Commands::Verify { action } => match action {
            VerifyAction::List { item_type, limit } => {
                cmd_verify_list(item_type.as_deref(), limit)?;
            }
            VerifyAction::Show { id } => {
                cmd_verify_show(&id)?;
            }
            VerifyAction::Approve { id, note } => {
                cmd_verify_approve(&id, note.as_deref())?;
            }
            VerifyAction::Reject { id, reason } => {
                cmd_verify_reject(&id, reason.as_deref())?;
            }
            VerifyAction::Stats => {
                cmd_verify_stats()?;
            }
            VerifyAction::Demo => {
                cmd_verify_demo()?;
            }
        },
    }

    Ok(())
}

/// Extract entities and relations from text
fn cmd_extract(input: &str, entities_only: bool, relations_only: bool) -> anyhow::Result<()> {
    let ner = RuleBasedNer::new();
    let re = RuleBasedRe::new();

    let entities = ner.extract(input)?;
    let relations = re.extract(input, &entities)?;

    if !entities_only {
        println!("\n=== Entities ===\n");
        if entities.is_empty() {
            println!("  (no entities found)");
        } else {
            for entity in &entities {
                println!(
                    "  [{:.2}] {}: \"{}\" @ {}..{}",
                    entity.confidence, entity.entity_type, entity.text, entity.start, entity.end
                );
            }
        }
    }

    if !relations_only {
        println!("\n=== Relations ===\n");
        if relations.is_empty() {
            println!("  (no relations found)");
        } else {
            for relation in &relations {
                println!(
                    "  [{:.2}] ({}) --[{}]--> ({})",
                    relation.confidence,
                    relation.subject.text,
                    relation.predicate,
                    relation.object.text
                );
            }
        }
    }

    Ok(())
}

/// List pending extractions
fn cmd_verify_list(item_type: Option<&str>, limit: usize) -> anyhow::Result<()> {
    let queue = VERIFICATION_QUEUE.lock().unwrap();

    let show_entities = item_type.is_none_or(|t| t == "entity" || t == "entities");
    let show_relations = item_type.is_none_or(|t| t == "relation" || t == "relations");

    if show_entities {
        let pending = queue.pending_entities();
        println!("\n=== Pending Entities ({}) ===\n", pending.len());

        for (i, entity) in pending.iter().take(limit).enumerate() {
            println!(
                "  {}. [{}] {}: \"{}\" (confidence: {:.2})",
                i + 1,
                &entity.id.to_string()[..8],
                entity.entity.entity_type,
                entity.entity.text,
                entity.entity.confidence
            );
        }

        if pending.len() > limit {
            println!("  ... and {} more", pending.len() - limit);
        }
    }

    if show_relations {
        let pending = queue.pending_relations();
        println!("\n=== Pending Relations ({}) ===\n", pending.len());

        for (i, rel) in pending.iter().take(limit).enumerate() {
            println!(
                "  {}. [{}] ({}) --[{}]--> ({}) (confidence: {:.2})",
                i + 1,
                &rel.id.to_string()[..8],
                rel.relation.subject.text,
                rel.relation.predicate,
                rel.relation.object.text,
                rel.relation.confidence
            );
        }

        if pending.len() > limit {
            println!("  ... and {} more", pending.len() - limit);
        }
    }

    Ok(())
}

/// Show item details
fn cmd_verify_show(id: &str) -> anyhow::Result<()> {
    let uuid = Uuid::parse_str(id)?;
    let queue = VERIFICATION_QUEUE.lock().unwrap();

    if let Some(entity) = queue.get_entity(uuid) {
        println!("\n=== Entity Details ===\n");
        println!("  ID:         {}", entity.id);
        println!("  Document:   {}", entity.document_id);
        println!("  Text:       \"{}\"", entity.entity.text);
        println!("  Type:       {}", entity.entity.entity_type);
        println!(
            "  Position:   {}..{}",
            entity.entity.start, entity.entity.end
        );
        println!("  Confidence: {:.2}", entity.entity.confidence);
        println!("  Status:     {}", entity.status);
        println!("  Created:    {}", entity.created_at);
        if let Some(reviewer) = &entity.reviewer {
            println!("  Reviewer:   {reviewer}");
        }
        if let Some(note) = &entity.review_note {
            println!("  Note:       {note}");
        }
    } else if let Some(rel) = queue.get_relation(uuid) {
        println!("\n=== Relation Details ===\n");
        println!("  ID:         {}", rel.id);
        println!("  Document:   {}", rel.document_id);
        println!(
            "  Triple:     ({}) --[{}]--> ({})",
            rel.relation.subject.text, rel.relation.predicate, rel.relation.object.text
        );
        println!("  Confidence: {:.2}", rel.relation.confidence);
        println!("  Status:     {}", rel.status);
        println!("  Created:    {}", rel.created_at);
        if let Some(reviewer) = &rel.reviewer {
            println!("  Reviewer:   {reviewer}");
        }
        if let Some(note) = &rel.review_note {
            println!("  Note:       {note}");
        }
    } else {
        println!("Item not found: {id}");
    }

    Ok(())
}

/// Approve an extraction
fn cmd_verify_approve(id: &str, note: Option<&str>) -> anyhow::Result<()> {
    let uuid = Uuid::parse_str(id)?;
    let mut queue = VERIFICATION_QUEUE.lock().unwrap();

    let reviewer = std::env::var("USER").unwrap_or_else(|_| "cli".to_string());

    if queue.approve_entity(uuid, &reviewer, note) {
        println!("Entity approved: {id}");
        return Ok(());
    }

    if queue.approve_relation(uuid, &reviewer, note) {
        println!("Relation approved: {id}");
        return Ok(());
    }

    println!("Item not found or already reviewed: {id}");
    Ok(())
}

/// Reject an extraction
fn cmd_verify_reject(id: &str, reason: Option<&str>) -> anyhow::Result<()> {
    let uuid = Uuid::parse_str(id)?;
    let mut queue = VERIFICATION_QUEUE.lock().unwrap();

    let reviewer = std::env::var("USER").unwrap_or_else(|_| "cli".to_string());
    let reason = reason.unwrap_or("No reason provided");

    if queue.reject_entity(uuid, &reviewer, reason) {
        println!("Entity rejected: {id}");
        return Ok(());
    }

    if queue.reject_relation(uuid, &reviewer, reason) {
        println!("Relation rejected: {id}");
        return Ok(());
    }

    println!("Item not found or already reviewed: {id}");
    Ok(())
}

/// Show verification statistics
fn cmd_verify_stats() -> anyhow::Result<()> {
    let queue = VERIFICATION_QUEUE.lock().unwrap();
    let stats = queue.stats();

    println!("\n=== Verification Statistics ===\n");
    println!("  Entities:");
    println!("    Pending:       {}", stats.pending_entities);
    println!("    Approved:      {}", stats.approved_entities);
    println!("    Auto-approved: {}", stats.auto_approved_entities);
    println!("    Rejected:      {}", stats.rejected_entities);
    println!("    Total:         {}", stats.total_entities());
    println!(
        "    Approval rate: {:.1}%",
        stats.entity_approval_rate() * 100.0
    );

    println!("\n  Relations:");
    println!("    Pending:       {}", stats.pending_relations);
    println!("    Approved:      {}", stats.approved_relations);
    println!("    Auto-approved: {}", stats.auto_approved_relations);
    println!("    Rejected:      {}", stats.rejected_relations);
    println!("    Total:         {}", stats.total_relations());
    println!(
        "    Approval rate: {:.1}%",
        stats.relation_approval_rate() * 100.0
    );

    Ok(())
}

/// Load demo data for testing
fn cmd_verify_demo() -> anyhow::Result<()> {
    let mut queue = VERIFICATION_QUEUE.lock().unwrap();
    let doc_id = Uuid::new_v4();

    // Sample HR text for extraction
    let sample_text =
        "연차휴가는 최대 15일까지 사용할 수 있습니다. 병가 신청에는 진단서가 필요합니다.
육아휴직은 최대 2년간 사용 가능합니다. 팀장 승인 후 인사팀에서 최종 결재합니다.";

    let ner = RuleBasedNer::new();
    let re = RuleBasedRe::new();

    let entities = ner.extract(sample_text)?;
    let relations = re.extract(sample_text, &entities)?;

    println!("Extracting from sample HR text...\n");
    println!("Text: {sample_text}\n");

    // Add entities to queue
    for entity in entities {
        let id = queue.add_entity(doc_id, entity.clone());
        println!(
            "Added entity: [{}] {} \"{}\" (conf: {:.2})",
            &id.to_string()[..8],
            entity.entity_type,
            entity.text,
            entity.confidence
        );
    }

    // Add relations to queue
    for relation in relations {
        let id = queue.add_relation(doc_id, relation.clone());
        println!(
            "Added relation: [{}] ({}) --[{}]--> ({}) (conf: {:.2})",
            &id.to_string()[..8],
            relation.subject.text,
            relation.predicate,
            relation.object.text,
            relation.confidence
        );
    }

    let stats = queue.stats();
    println!(
        "\nDemo loaded: {} entities ({} pending), {} relations ({} pending)",
        stats.total_entities(),
        stats.pending_entities,
        stats.total_relations(),
        stats.pending_relations
    );

    Ok(())
}

/// Query the knowledge base using RAG
async fn cmd_query(
    question: &str,
    stream: bool,
    use_ollama: bool,
    model: Option<&str>,
) -> anyhow::Result<()> {
    // Determine LLM to use
    let llm_client: Box<dyn LlmClient> = if use_ollama {
        let model = model.unwrap_or("llama2");
        println!("Using Ollama with model: {model}");
        Box::new(OllamaClient::new("http://localhost:11434", model))
    } else if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        let model = model.unwrap_or("gpt-4o-mini");
        println!("Using OpenAI with model: {model}");
        Box::new(otl_rag::OpenAiClient::new(&api_key, model, 2048, 0.1))
    } else {
        println!("Note: No OPENAI_API_KEY found, falling back to Ollama");
        let model = model.unwrap_or("llama2");
        Box::new(OllamaClient::new("http://localhost:11434", model))
    };

    println!("\nQuestion: {question}\n");
    println!("---");

    // Build a simple prompt (in production, this would include RAG context)
    let prompt = format!(
        r#"당신은 조직의 지식 전문가입니다.
다음 질문에 한국어로 답변해 주세요.

질문: {question}

답변:"#
    );

    if stream {
        // Streaming response
        println!();
        match llm_client.generate_stream(&prompt).await {
            Ok(mut stream) => {
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(chunk) => {
                            print!("{chunk}");
                            io::stdout().flush()?;
                        }
                        Err(e) => {
                            eprintln!("\nStream error: {e}");
                            break;
                        }
                    }
                }
                println!("\n");
            }
            Err(e) => {
                eprintln!("Failed to start stream: {e}");
            }
        }
    } else {
        // Regular response
        match llm_client.generate(&prompt).await {
            Ok(response) => {
                println!("\n{response}\n");
            }
            Err(e) => {
                eprintln!("Error: {e}");
            }
        }
    }

    println!("---");
    Ok(())
}
