use anyhow::{Context, Result};
use colored::Colorize;
use std::io::{self, BufRead, IsTerminal};

use crate::cli::prompt_passphrase;
use crate::store::Store;

/// Parse a line into key-value pair
/// Supports multiple formats:
/// - KEY=VALUE (dotenv style)
/// - KEY: VALUE (heroku config style)
/// - KEY:VALUE (compact)
/// - export KEY=VALUE (shell export)
fn parse_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();

    // Skip empty lines and comments
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    // Remove 'export ' prefix if present
    let line = line.strip_prefix("export ").unwrap_or(line);

    // Try KEY=VALUE first
    if let Some((key, value)) = line.split_once('=') {
        let key = key.trim();
        let value = value.trim();
        // Remove surrounding quotes if present
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(value);

        if !key.is_empty() {
            return Some((key.to_string(), value.to_string()));
        }
    }

    // Try KEY: VALUE (heroku style)
    if let Some((key, value)) = line.split_once(':') {
        let key = key.trim();
        let value = value.trim();

        if !key.is_empty() && !key.contains(' ') {
            return Some((key.to_string(), value.to_string()));
        }
    }

    None
}

pub fn run(project: &str, environment: &str, file: Option<&str>) -> Result<()> {
    // Check if we have input
    let stdin = io::stdin();

    if file.is_none() && stdin.is_terminal() {
        anyhow::bail!(
            "No input provided. Pipe data or specify a file:\n\
             \n\
             Examples:\n\
             \x20 heroku config | ts import-env myapp staging\n\
             \x20 cat .env | ts import-env myapp staging\n\
             \x20 ts import-env myapp staging -f .env.example"
        );
    }

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let lines: Vec<String> = match file {
        Some(path) => std::fs::read_to_string(path)
            .context(format!("Failed to read file: {}", path))?
            .lines()
            .map(String::from)
            .collect(),
        None => stdin
            .lock()
            .lines()
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to read from stdin")?,
    };

    let mut imported = 0;
    let mut skipped = 0;

    for line in lines {
        if let Some((key, value)) = parse_line(&line) {
            store.set(project, environment, &key, &value, None)?;
            eprintln!("  {} {}", "✓".green(), key.bold());
            imported += 1;
        } else if !line.trim().is_empty() && !line.trim().starts_with('#') {
            eprintln!(
                "  {} {} (couldn't parse)",
                "○".yellow(),
                line.trim().dimmed()
            );
            skipped += 1;
        }
    }

    eprintln!();
    if imported > 0 {
        eprintln!(
            "{} Imported {} secrets into {}/{}",
            "✓".green(),
            imported.to_string().bold(),
            project.cyan(),
            environment.yellow()
        );
    }
    if skipped > 0 {
        eprintln!("{} Skipped {} unparseable lines", "○".yellow(), skipped);
    }
    if imported == 0 && skipped == 0 {
        eprintln!("{} No secrets found in input", "○".yellow());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dotenv() {
        assert_eq!(
            parse_line("DATABASE_URL=postgres://localhost/db"),
            Some((
                "DATABASE_URL".to_string(),
                "postgres://localhost/db".to_string()
            ))
        );
    }

    #[test]
    fn test_parse_quoted() {
        assert_eq!(
            parse_line("API_KEY=\"sk-secret-key\""),
            Some(("API_KEY".to_string(), "sk-secret-key".to_string()))
        );
        assert_eq!(
            parse_line("API_KEY='sk-secret-key'"),
            Some(("API_KEY".to_string(), "sk-secret-key".to_string()))
        );
    }

    #[test]
    fn test_parse_heroku() {
        assert_eq!(
            parse_line("DATABASE_URL: postgres://localhost/db"),
            Some((
                "DATABASE_URL".to_string(),
                "postgres://localhost/db".to_string()
            ))
        );
    }

    #[test]
    fn test_parse_export() {
        assert_eq!(
            parse_line("export API_KEY=secret"),
            Some(("API_KEY".to_string(), "secret".to_string()))
        );
    }

    #[test]
    fn test_skip_comments() {
        assert_eq!(parse_line("# this is a comment"), None);
        assert_eq!(parse_line(""), None);
        assert_eq!(parse_line("   "), None);
    }

    #[test]
    fn test_value_with_equals() {
        // Values can contain = signs (common in URLs, base64, etc.)
        assert_eq!(
            parse_line("DATABASE_URL=postgres://user:pass@host/db?ssl=true"),
            Some((
                "DATABASE_URL".to_string(),
                "postgres://user:pass@host/db?ssl=true".to_string()
            ))
        );
    }

    #[test]
    fn test_value_with_spaces() {
        assert_eq!(
            parse_line("MESSAGE=\"Hello World\""),
            Some(("MESSAGE".to_string(), "Hello World".to_string()))
        );
    }

    #[test]
    fn test_empty_value() {
        assert_eq!(
            parse_line("EMPTY_VAR="),
            Some(("EMPTY_VAR".to_string(), "".to_string()))
        );
    }
}
