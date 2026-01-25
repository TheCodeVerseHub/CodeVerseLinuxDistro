//! CVH Icons - Sandboxed Lua-scriptable file/folder icon system
//!
//! Displays desktop icons for files and folders with customizable
//! Lua scripts for rendering and behavior.

use anyhow::Result;
use clap::Parser;
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod daemon;
mod icons;
mod lua;
mod renderer;
mod sandbox;
mod wayland;

/// CVH Icons - Desktop icon manager
#[derive(Parser, Debug)]
#[command(name = "cvh-icons")]
#[command(author = "CVH Linux Team")]
#[command(version = "0.1.0")]
#[command(about = "Sandboxed Lua-scriptable desktop icons")]
struct Args {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<std::path::PathBuf>,

    /// Desktop directory to display icons for
    #[arg(short, long)]
    desktop: Option<std::path::PathBuf>,

    /// Run in daemon mode
    #[arg(short = 'D', long)]
    daemon: bool,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// List available icon scripts
    #[arg(long)]
    list_scripts: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let filter = if args.verbose {
        "cvh_icons=debug,warn"
    } else {
        "cvh_icons=info,warn"
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::new(filter))
        .init();

    info!("CVH Icons v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = config::Config::load(args.config.as_deref())?;

    if args.list_scripts {
        // List available Lua scripts
        list_scripts(&config)?;
        return Ok(());
    }

    // Determine desktop directory
    let desktop_dir = args.desktop
        .or_else(|| dirs::desktop_dir())
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join("Desktop"))
                .unwrap_or_else(|| std::path::PathBuf::from("."))
        });

    info!("Desktop directory: {}", desktop_dir.display());

    // Initialize the daemon
    let mut daemon = daemon::IconDaemon::new(config, desktop_dir).await?;

    // Run the main loop
    daemon.run().await?;

    Ok(())
}

fn list_scripts(config: &config::Config) -> Result<()> {
    println!("Available icon scripts:");
    println!();

    for script_dir in &config.script_dirs {
        if !script_dir.exists() {
            continue;
        }

        for entry in std::fs::read_dir(script_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |e| e == "lua") {
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");

                println!("  - {}", name);
            }
        }
    }

    Ok(())
}
