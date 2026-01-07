mod cli;
mod config;
mod crypto;
mod keychain;
mod store;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, ConfigAction};
use config::ConfigResolver;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => cli::init::run()?,
        Commands::Set {
            project,
            environment,
            key,
            value,
        } => {
            let resolver = ConfigResolver::new()?;
            let project = resolver.project(project.as_deref())?;
            let environment = resolver.environment(environment.as_deref())?;
            cli::set::run(&project, &environment, &key, value.as_deref())?
        }
        Commands::Get {
            project,
            environment,
            key,
            version,
        } => {
            let resolver = ConfigResolver::new()?;
            let project = resolver.project(project.as_deref())?;
            let environment = resolver.environment(environment.as_deref())?;
            cli::get::run(&project, &environment, &key, version)?
        }
        Commands::List {
            project,
            environment,
        } => {
            // List can work without project/env (shows all), but use config as default filter
            let resolver = ConfigResolver::new()?;
            let project = project.or_else(|| {
                resolver
                    .config()
                    .and_then(|c| c.project.clone())
            });
            let environment = environment.or_else(|| {
                resolver
                    .config()
                    .and_then(|c| c.environment.clone())
            });
            cli::list::run(project.as_deref(), environment.as_deref())?
        }
        Commands::Delete {
            project,
            environment,
            key,
        } => {
            let resolver = ConfigResolver::new()?;
            let project = resolver.project(project.as_deref())?;
            let environment = resolver.environment(environment.as_deref())?;
            cli::delete::run(&project, &environment, &key)?
        }
        Commands::Run {
            project,
            environment,
            command,
        } => {
            let resolver = ConfigResolver::new()?;
            let project = resolver.project(project.as_deref())?;
            let environment = resolver.environment(environment.as_deref())?;
            cli::run::run(&project, &environment, &command)?
        }
        Commands::Export {
            project,
            environment,
            output,
        } => {
            let resolver = ConfigResolver::new()?;
            let project = resolver.project(project.as_deref())?;
            let environment = resolver.environment(environment.as_deref())?;
            cli::export::run(&project, &environment, output.as_deref())?
        }
        Commands::Import { input } => cli::import::run(&input)?,
        Commands::ImportEnv {
            project,
            environment,
            file,
        } => {
            let resolver = ConfigResolver::new()?;
            let project = resolver.project(project.as_deref())?;
            let environment = resolver.environment(environment.as_deref())?;
            cli::import_env::run(&project, &environment, file.as_deref())?
        }
        Commands::History {
            project,
            environment,
            key,
            limit,
            show,
        } => {
            let resolver = ConfigResolver::new()?;
            let project = resolver.project(project.as_deref())?;
            let environment = resolver.environment(environment.as_deref())?;
            cli::history::run(&project, &environment, &key, limit, show)?
        }
        Commands::Projects => cli::projects::run()?,
        Commands::Envs { project } => {
            let resolver = ConfigResolver::new()?;
            let project = resolver.project(project.as_deref())?;
            cli::envs::run(&project)?
        }
        Commands::Keychain { action } => cli::keychain_cmd::run(action)?,
        Commands::Config { action } => match action {
            ConfigAction::Init {
                project,
                environment,
            } => cli::config::run_init(&project, environment.as_deref())?,
            ConfigAction::Show => cli::config::run_show()?,
            ConfigAction::Set {
                project,
                environment,
            } => cli::config::run_set(project.as_deref(), environment.as_deref())?,
        },
        Commands::Examples => cli::examples::run(),
    }

    Ok(())
}
