mod claude;
mod config;
mod context;
mod git;
mod path;
mod prompt;
mod session;

use config::Config;
use std::env;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
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

    if args.contains(&"--version".to_string()) {
        println!("claude-idr {VERSION}");
        return;
    }

    let config_path = args
        .windows(2)
        .find(|w| w[0] == "--config")
        .map(|w| std::path::Path::new(&w[1]));

    let dry_run = args.contains(&"--dry-run".to_string());

    let config = Config::load(config_path);
    if !config.enabled {
        eprintln!("claude-idr: disabled by config");
        return;
    }

    // Check for session with Write/Edit activity
    let session_path = session::find_recent(&config);
    let has_changes = session_path
        .as_ref()
        .map(|p| session::has_write_or_edit(p))
        .unwrap_or(false);
    if !has_changes {
        eprintln!("claude-idr: no session with Write/Edit activity found");
        return;
    }

    // Get staged diff
    let diff = git::staged_diff();
    let stat = git::staged_stat();
    if diff.is_empty() {
        eprintln!("claude-idr: no staged changes");
        return;
    }

    if dry_run {
        let idr_prompt = prompt::build_idr_prompt(&diff, &stat, &config);
        eprintln!("claude-idr: dry-run mode");
        eprintln!("--- IDR prompt ({} chars) ---", idr_prompt.len());
        eprintln!("{idr_prompt}");
        return;
    }

    // Extract context and purpose
    let purpose = session_path
        .as_ref()
        .and_then(|p| context::extract(p))
        .and_then(|ctx| {
            let purpose_prompt = prompt::build_purpose_prompt(&ctx, &config);
            claude::run(&purpose_prompt, &config)
        })
        .map(|s| s.trim().to_string());

    // Generate IDR content
    eprintln!("claude-idr: generating IDR...");
    let idr_prompt = prompt::build_idr_prompt(&diff, &stat, &config);
    let idr_content = claude::run(&idr_prompt, &config)
        .unwrap_or_else(|| "## 変更概要\n\n(IDR生成失敗 - 手動で記載してください)".to_string());

    // Write IDR file
    let output_dir = path::resolve(&config);
    let next_num = path::next_number(&output_dir);
    let output_file = output_dir.join(format!("idr-{:02}.md", next_num));

    path::write_idr(&output_file, &purpose, &idr_content, &stat);
    eprintln!("claude-idr: IDR generated: {}", output_file.display());
}
