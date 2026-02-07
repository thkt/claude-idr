mod claude;
mod config;
mod context;
mod git;
mod jsonl;
mod path;
mod prompt;
mod session;
#[cfg(test)]
mod testutil;

use config::Config;
use std::env;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    run(&env::args().collect::<Vec<_>>());
}

fn run(args: &[String]) {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage: claude-idr [OPTIONS]");
        println!();
        println!("Generate Implementation Decision Records from git diffs using Claude.");
        println!();
        println!("Options:");
        println!("  --config <PATH>  Config file path");
        println!("  --dry-run        Show prompt without calling claude");
        println!("  --version        Show version");
        println!("  --help           Show help");
        return;
    }

    if args.iter().any(|a| a == "--version") {
        println!("claude-idr {VERSION}");
        return;
    }

    let config_path = args
        .windows(2)
        .find(|w| w[0] == "--config")
        .map(|w| std::path::Path::new(&w[1]));

    let dry_run = args.iter().any(|a| a == "--dry-run");

    let config = Config::load(config_path);
    if !config.enabled {
        eprintln!("claude-idr: disabled by config");
        return;
    }

    let session_path = match session::find_recent(&config) {
        None => {
            eprintln!("claude-idr: no recent session found");
            return;
        }
        Some(p) if !session::has_write_or_edit(&p) => {
            eprintln!(
                "claude-idr: session found but no code changes via Claude detected: {}",
                p.display()
            );
            return;
        }
        Some(p) => p,
    };

    let diff = match git::staged_diff() {
        None => {
            eprintln!("claude-idr: git failed");
            return;
        }
        Some(d) if d.is_empty() => {
            eprintln!("claude-idr: no staged changes");
            return;
        }
        Some(d) => d,
    };
    let stat = git::staged_stat();

    let changed_lines = git::staged_changed_lines();
    if changed_lines > config.max_diff_lines {
        eprintln!(
            "claude-idr: diff too large ({changed_lines} lines > {} limit), skipping. Split your commit for IDR generation.",
            config.max_diff_lines
        );
        return;
    }

    if dry_run {
        let idr_prompt = prompt::build_idr_prompt(&diff, &stat, &config);
        eprintln!("claude-idr: dry-run mode");
        eprintln!("--- IDR prompt ({} chars) ---", idr_prompt.len());
        eprintln!("{idr_prompt}");
        return;
    }

    let purpose = context::extract(&session_path)
        .and_then(|ctx| {
            let purpose_prompt = prompt::build_purpose_prompt(&ctx, &config);
            claude::run(&purpose_prompt, &config)
        })
        .map(|s| s.trim().to_string());

    eprintln!("claude-idr: generating IDR...");
    let idr_prompt = prompt::build_idr_prompt(&diff, &stat, &config);
    let idr_content = claude::run(&idr_prompt, &config)
        .unwrap_or_else(|| "## 変更概要\n\n(IDR生成失敗 - 手動で記載してください)".to_string());

    let output_dir = path::resolve(&config);
    let next_num = path::next_number(&output_dir);
    let output_file = output_dir.join(format!("idr-{:02}.md", next_num));

    path::write_idr(&output_file, &purpose, &idr_content, &stat);
    eprintln!("claude-idr: IDR generated: {}", output_file.display());
}
