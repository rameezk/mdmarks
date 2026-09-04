use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use mdmarks::add::{add, AddOutcome};
use mdmarks::config::{resolve_store_path, Config};
use mdmarks::import::import;
use mdmarks::json;
use mdmarks::list::{bookmarks_in_space, list, render_line};
use mdmarks::open::{open, SpaceResolver, SystemLauncher};
use mdmarks::rm::rm;
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
    Import {
        file: PathBuf,
    },
    Rm {
        url: String,
    },
    Open {
        url: String,
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
        Command::Import { file } => {
            let store_path = resolve_store_path().map_err(|e| e.to_string())?;
            let store = Store::new(store_path);
            let summary = import(&store, &file).map_err(|e| e.to_string())?;
            println!(
                "Imported {}",
                quantity(summary.imported, "bookmark", "bookmarks")
            );
            println!(
                "Skipped {} and {}",
                quantity(summary.duplicates.len(), "duplicate", "duplicates"),
                quantity(
                    summary.unparseable,
                    "unparseable entry",
                    "unparseable entries"
                ),
            );
            if !summary.duplicates.is_empty() {
                println!("Duplicates skipped:");
                for dup in &summary.duplicates {
                    println!("  {}  {}", dup.title, dup.url);
                }
            }
            Ok(())
        }
        Command::Rm { url } => {
            let store_path = resolve_store_path().map_err(|e| e.to_string())?;
            let store = Store::new(store_path);
            let removed = rm(&store, &url).map_err(|e| e.to_string())?;
            println!("Removed \"{}\"", removed.frontmatter.display_title());
            println!("  {}", removed.frontmatter.url);
            println!("  {}", removed.path.display());
            Ok(())
        }
        Command::Open { url, space } => {
            let config = Config::load().map_err(|e| e.to_string())?;
            let store = Store::new(&config.store);
            let resolver = SpaceResolver {
                override_space: space.as_deref(),
                default_space: config.default_space.as_deref(),
                spaces: &config.spaces,
            };
            let opened =
                open(&store, &url, &resolver, &SystemLauncher).map_err(|e| e.to_string())?;
            println!("Opening \"{}\"", opened.frontmatter.display_title());
            println!("  {}", opened.frontmatter.url);
            Ok(())
        }
    }
}

fn quantity(n: usize, singular: &str, plural: &str) -> String {
    format!("{n} {}", if n == 1 { singular } else { plural })
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
