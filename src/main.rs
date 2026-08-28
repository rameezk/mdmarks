use std::process::ExitCode;

use clap::{Parser, Subcommand};

use mdmarks::add::{add, AddOutcome};
use mdmarks::config::resolve_store_path;
use mdmarks::json;
use mdmarks::list::{bookmarks_in_space, list, render_line};
use mdmarks::search::rank;
use mdmarks::store::{Store, StoredBookmark};

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
    List {
        #[arg(long = "json")]
        as_json: bool,
        #[arg(long)]
        space: Option<String>,
    },
    Search {
        query: String,
        #[arg(long = "json")]
        as_json: bool,
        #[arg(long)]
        space: Option<String>,
    },
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
        Command::List { as_json, space } => {
            let store_path = resolve_store_path().map_err(|e| e.to_string())?;
            let store = Store::new(store_path);
            let bookmarks = list(&store, space.as_deref()).map_err(|e| e.to_string())?;
            let results: Vec<&StoredBookmark> = bookmarks.iter().collect();
            render(&results, as_json);
            Ok(())
        }
        Command::Search {
            query,
            as_json,
            space,
        } => {
            let store_path = resolve_store_path().map_err(|e| e.to_string())?;
            let store = Store::new(store_path);
            let bookmarks =
                bookmarks_in_space(&store, space.as_deref()).map_err(|e| e.to_string())?;
            let results = rank(&bookmarks, &query);
            render(&results, as_json);
            Ok(())
        }
    }
}

fn render(results: &[&StoredBookmark], as_json: bool) {
    if as_json {
        println!("{}", json::render(results));
    } else {
        for bookmark in results {
            println!("{}", render_line(bookmark));
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
