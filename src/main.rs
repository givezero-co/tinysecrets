mod cli;
mod config;
mod crypto;
mod keychain;
mod keypath;
mod store;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, ComposeAction, ConfigAction, PackAction};
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
            let resolver = ConfigResolver::new()?;
            let project = project.or_else(|| resolver.config().and_then(|c| c.project.clone()));
            let environment =
                environment.or_else(|| resolver.config().and_then(|c| c.environment.clone()));
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
            with,
            compose,
            command,
        } => {
            let resolver = ConfigResolver::new()?;
            let project = resolver.project(project.as_deref())?;
            let environment = resolver.environment(environment.as_deref())?;

            let config_compose = resolver.config().and_then(|c| c.compose.clone());
            cli::run::run(
                &project,
                &environment,
                &command,
                config_compose.as_deref(),
                &with,
                compose.as_deref(),
            )?
        }
        Commands::Pack { action } => {
            let resolver = ConfigResolver::new()?;
            match action {
                PackAction::Set {
                    project,
                    environment,
                    pack,
                    entries,
                } => cli::pack::run_set(
                    &resolver,
                    project.as_deref(),
                    environment.as_deref(),
                    &pack,
                    &entries,
                )?,
                PackAction::Get {
                    project,
                    environment,
                    pack,
                    key,
                } => cli::pack::run_get(
                    &resolver,
                    project.as_deref(),
                    environment.as_deref(),
                    &pack,
                    &key,
                )?,
                PackAction::Show {
                    project,
                    environment,
                    pack,
                    reveal,
                } => cli::pack::run_show(
                    &resolver,
                    project.as_deref(),
                    environment.as_deref(),
                    &pack,
                    reveal,
                )?,
                PackAction::List {
                    project,
                    environment,
                    keys,
                } => cli::pack::run_list(
                    &resolver,
                    project.as_deref(),
                    environment.as_deref(),
                    keys,
                )?,
                PackAction::Clone {
                    project,
                    environment,
                    source,
                    destination,
                    force,
                } => cli::pack::run_clone(
                    &resolver,
                    project.as_deref(),
                    environment.as_deref(),
                    &source,
                    &destination,
                    force,
                )?,
                PackAction::Delete {
                    project,
                    environment,
                    pack,
                    yes,
                } => cli::pack::run_delete(
                    &resolver,
                    project.as_deref(),
                    environment.as_deref(),
                    &pack,
                    yes,
                )?,
                PackAction::History {
                    project,
                    environment,
                    pack,
                    key,
                    limit,
                } => cli::pack::run_history(
                    &resolver,
                    project.as_deref(),
                    environment.as_deref(),
                    &pack,
                    &key,
                    limit,
                )?,
                PackAction::Group {
                    project,
                    environment,
                    yes,
                    dry_run,
                    min_size,
                } => cli::pack::run_group(
                    &resolver,
                    project.as_deref(),
                    environment.as_deref(),
                    yes,
                    dry_run,
                    min_size,
                )?,
                PackAction::Adopt {
                    project,
                    environment,
                    pack,
                    keys,
                } => cli::pack::run_adopt(
                    &resolver,
                    project.as_deref(),
                    environment.as_deref(),
                    &pack,
                    &keys,
                )?,
                PackAction::Move {
                    project,
                    environment,
                    source,
                    destination,
                    keys,
                } => cli::pack::run_move(
                    &resolver,
                    project.as_deref(),
                    environment.as_deref(),
                    &source,
                    &destination,
                    &keys,
                )?,
            }
        }
        Commands::Compose { action } => {
            let resolver = ConfigResolver::new()?;
            match action {
                ComposeAction::Show {
                    project,
                    environment,
                    reveal,
                } => cli::compose::run_show(
                    &resolver,
                    project.as_deref(),
                    environment.as_deref(),
                    reveal,
                )?,
                ComposeAction::Check {
                    project,
                    environment,
                } => {
                    cli::compose::run_check(&resolver, project.as_deref(), environment.as_deref())?
                }
            }
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
        Commands::Migrate => cli::migrate::run()?,
    }

    Ok(())
}
