use crate::prelude::*;

pub fn run() -> Result<()> {
    let args: SyncYnab = SyncYnab::from_args();

    match args.command {
        SyncYnabCommands::Config(n) => config::handle(args.args, n),
        SyncYnabCommands::Sync(_n) => {
            crate::ynab::sync(&mut crate::config::load_config(args.args.config_directory)?)
        }
    }
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct SyncYnab {
    #[structopt(flatten)]
    pub args: SyncYnabArgs,
    #[structopt(subcommand)]
    pub command: SyncYnabCommands,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct SyncYnabArgs {
    #[structopt(long, default_value = "secrets/")]
    pub config_directory: String,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum SyncYnabCommands {
    Config(config::ConfigCommands),
    Sync(sync::SyncArgs),
}

pub mod config {
    use crate::cli::SyncYnabArgs;
    use crate::config::*;
    use crate::prelude::*;
    use std::io::{stdin, BufRead};

    #[derive(StructOpt)]
    #[structopt(rename_all = "kebab-case")]
    pub enum ConfigCommands {
        TestProviders,
        TestYnab,
        AddTruelayer,
    }

    pub fn handle(args: SyncYnabArgs, command: ConfigCommands) -> Result<()> {
        use anyhow::ensure;

        let mut config = crate::config::load_config(&args.config_directory)?;

        use oauth2::TokenResponse;

        match command {
            ConfigCommands::TestProviders => {
                let mut result = crate::load_connections(&mut config)?;
                for provider in &mut result {
                    let accounts = provider.as_mut().get_accounts()?;
                    println!("{:#?}", accounts);
                    for acc in accounts {
                        let trans = provider.as_mut().get_transactions(&acc)?;
                        println!("{} has {} transactions", acc.display_name, trans.len(),);
                    }
                }
            }
            ConfigCommands::TestYnab => {
                ensure!(
                    !&config.ynab_config.access_token.is_empty(),
                    "access_token for YNAB must be set in config"
                );

                let mut rc = crate::ynab::new_rest_client(&config.ynab_config.access_token);
                println!(
                    "Getting accounts in YNAB budget_id: {}",
                    &config.ynab_config.budget_id
                );
                println!(
                    "{:#?}",
                    crate::ynab::get_accounts(&mut rc, &config.ynab_config.budget_id)
                );
            }
            ConfigCommands::AddTruelayer => {
                if config.ynab_config.truelayer_client_id.is_empty() {
                    println!("Missing truelayer client ID. Enter truelayer client ID:");
                    config.ynab_config.truelayer_client_id = read_line()?.trim().to_string();
                }

                if config.ynab_config.truelayer_client_secret.is_empty() {
                    println!("Missing truelayer client secret. Enter truelayer client secret:");
                    config.ynab_config.truelayer_client_secret = read_line()?.trim().to_string();
                }

                println!(
                    "Please authenticate at:\n{}",
                    crate::truelayer::get_auth_url(&config.ynab_config)?
                );

                println!("Enter code:\n");
                for line in stdin().lock().lines() {
                    let line = line?;
                    let line = line.trim();

                    if !line.is_empty() {
                        let token = crate::truelayer::authorize(&config.ynab_config, line)?;

                        let mut token = crate::truelayer::Token {
                            display_name: "unknown".to_string(),
                            access_token: token.access_token().clone(),
                            access_token_expiry: crate::truelayer::calculate_expiry_time(
                                token.expires_in().unwrap(),
                            ),
                            refresh_token: token.refresh_token().unwrap().clone(),
                        };

                        let (_refresh, result) =
                            crate::truelayer::initialize(&config.ynab_config, &mut token);
                        result?;
                        println!("Connected");

                        config.providers.push(Provider::Truelayer(token));

                        break;
                    }
                }

                crate::config::save_config(&args.config_directory, &config)?;
            }
        }
        Ok(())
    }

    fn read_line() -> Result<String> {
        stdin()
            .lock()
            .lines()
            .next()
            .ok_or_else(|| anyhow!("Couldn't read line from stdin"))?
            .map_err(|e| e.into())
    }
}

pub mod sync {
    use crate::prelude::*;

    #[derive(StructOpt)]
    #[structopt(rename_all = "kebab-case")]
    pub struct SyncArgs {
        //        #[structopt(subcommand)]
        //        command: SyncCommands
    }

    //    #[derive(StructOpt)]
    //    #[structopt(rename_all = "kebab-case")]
    //    pub enum SyncCommands {
    //
    //    }
}
