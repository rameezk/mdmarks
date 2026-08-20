use std::process::ExitCode;

use clap::{Parser, Subcommand};

use mdmarks::add::{add, AddOutcome};
use mdmarks::config::resolve_store_path;
use mdmarks::list::{list, render_line};
use mdmarks::store::Store;

#[derive(Parser)]
#[command(
    name = "mdmarks",
    version,
    about = "A markdown-driven bookmark manager"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Add {
        url: String,
        #[arg(long)]
        title: Option<String>,
    },
    List,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Add { url, title } => {
            let store_path = resolve_store_path().map_err(|e| e.to_string())?;
            let store = Store::new(store_path);
            let outcome = add(&store, &url, title.as_deref()).map_err(|e| e.to_string())?;
            report(&outcome);
            Ok(())
        }
        Command::List => {
            let store_path = resolve_store_path().map_err(|e| e.to_string())?;
            let store = Store::new(store_path);
            let bookmarks = list(&store).map_err(|e| e.to_string())?;
            for bookmark in &bookmarks {
                println!("{}", render_line(bookmark));
            }
            Ok(())
        }
    }
}

fn report(outcome: &AddOutcome) {
    let (label, bookmark) = match outcome {
        AddOutcome::Created(b) => ("Added", b),
        AddOutcome::Matched(b) => ("Already saved as", b),
    };
    println!("{label} \"{}\"", bookmark.title);
    println!("  {}", bookmark.url);
    println!("  {}", bookmark.path.display());
}
