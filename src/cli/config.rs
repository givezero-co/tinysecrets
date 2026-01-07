use anyhow::Result;
use colored::Colorize;

use crate::config::Config;

pub fn run_init(project: &str, environment: Option<&str>) -> Result<()> {
    let path = Config::config_path()?;

    if path.exists() {
        eprintln!(
            "{} Config file already exists at {}",
            "⚠".yellow(),
            path.display().to_string().cyan()
        );
        eprintln!("  Use `tinysecrets config show` to view or `tinysecrets config set` to modify");
        return Ok(());
    }

    let saved_path = Config::init(project, environment)?;

    eprintln!(
        "{} Created {} with:",
        "✓".green(),
        saved_path.display().to_string().cyan()
    );
    eprintln!("  project: {}", project.yellow());
    if let Some(env) = environment {
        eprintln!("  environment: {}", env.yellow());
    }

    eprintln!();
    eprintln!(
        "{}",
        "You can now run commands without specifying -p/--project".dimmed()
    );

    Ok(())
}

pub fn run_show() -> Result<()> {
    match Config::found_path()? {
        Some(path) => {
            eprintln!(
                "{} {}",
                "Config file:".dimmed(),
                path.display().to_string().cyan()
            );
            eprintln!();

            let config = Config::load()?.unwrap();

            if let Some(project) = &config.project {
                eprintln!("  project: {}", project.yellow());
            }
            if let Some(environment) = &config.environment {
                eprintln!("  environment: {}", environment.yellow());
            }

            if config.project.is_none() && config.environment.is_none() {
                eprintln!("  {}", "(empty config)".dimmed());
            }
        }
        None => {
            eprintln!(
                "{} No .tinysecrets.toml found in current directory or ancestors",
                "⚠".yellow()
            );
            eprintln!();
            eprintln!(
                "Create one with: {}",
                "tinysecrets config init <project> [environment]".cyan()
            );
        }
    }

    Ok(())
}

pub fn run_set(project: Option<&str>, environment: Option<&str>) -> Result<()> {
    let mut config = Config::load()?.unwrap_or_default();
    let mut changed = false;

    if let Some(p) = project {
        config.project = Some(p.to_string());
        changed = true;
    }

    if let Some(e) = environment {
        config.environment = Some(e.to_string());
        changed = true;
    }

    if !changed {
        eprintln!("{} No changes specified", "⚠".yellow());
        eprintln!("Usage: tinysecrets config set [--project <name>] [--environment <name>]");
        return Ok(());
    }

    let path = config.save()?;

    eprintln!(
        "{} Updated {}",
        "✓".green(),
        path.display().to_string().cyan()
    );

    if let Some(p) = &config.project {
        eprintln!("  project: {}", p.yellow());
    }
    if let Some(e) = &config.environment {
        eprintln!("  environment: {}", e.yellow());
    }

    Ok(())
}
