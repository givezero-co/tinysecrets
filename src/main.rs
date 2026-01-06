mod cli;
mod crypto;
mod store;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => cli::init::run()?,
        Commands::Set { project, environment, key, value } => {
            cli::set::run(&project, &environment, &key, value.as_deref())?
        }
        Commands::Get { project, environment, key } => {
            cli::get::run(&project, &environment, &key)?
        }
        Commands::List { project, environment } => {
            cli::list::run(project.as_deref(), environment.as_deref())?
        }
        Commands::Delete { project, environment, key } => {
            cli::delete::run(&project, &environment, &key)?
        }
        Commands::Run { project, environment, command } => {
            cli::run::run(&project, &environment, &command)?
        }
        Commands::Export { project, environment, output } => {
            cli::export::run(&project, &environment, output.as_deref())?
        }
        Commands::Import { input } => {
            cli::import::run(&input)?
        }
        Commands::ImportEnv { project, environment, file } => {
            cli::import_env::run(&project, &environment, file.as_deref())?
        }
        Commands::History { project, environment, key, limit } => {
            cli::history::run(&project, &environment, &key, limit)?
        }
        Commands::Projects => {
            cli::projects::run()?
        }
        Commands::Envs { project } => {
            cli::envs::run(&project)?
        }
    }

    Ok(())
}

