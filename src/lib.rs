pub mod cli;
pub mod config;
pub mod currency;
pub mod revolut;
pub mod serialisation;
pub mod truelayer;
pub mod ynab;

pub mod prelude {
    pub use anyhow::{anyhow, Context, Result};
    pub use restson::{RestClient, RestPath};
    pub use serde::{Deserialize, Serialize};
    pub use std::collections::HashMap;
    pub use structopt::StructOpt;

    pub type UtcDate = chrono::Date<chrono::Utc>;
    pub type UtcDateTime = chrono::DateTime<chrono::Utc>;
}

#[derive(Debug)]
pub struct Account {
    pub account_id: String,
    pub currency: String,
    pub display_name: String,
    pub ty: AccountType,
    pub balance: i64,
}

#[derive(Debug)]
pub enum AccountType {
    Account,
    Card,
}

#[derive(Debug)]
pub struct Transaction {
    pub transaction_id: String,
    pub timestamp: UtcDateTime,
    pub amount: i64,
    pub description: String,
    pub payee_name: Option<String>,
    pub category: Option<String>,
}

use crate::config::Config;
use crate::prelude::*;

pub trait ConnectedProvider: std::fmt::Debug {
    fn get_accounts(&mut self) -> Result<Vec<Account>>;
    fn get_transactions(&mut self, acc: &Account) -> Result<Vec<crate::Transaction>>;
}

/*
fn load_connections(
    cfg: &mut Config,
) -> Result<(Vec<Box<dyn ConnectedProvider>>, Vec<Error>), Error> {
*/

fn load_connections(cfg: &mut Config) -> Result<Vec<Box<dyn ConnectedProvider>>> {
    let mut connected: Vec<Box<dyn ConnectedProvider>> = vec![];

    let provider_count = cfg.providers.len();
    for idx in 0..provider_count {
        let (refreshed, connection) = match &mut cfg.providers[idx] {
            config::Provider::Truelayer(token) => truelayer::initialize(&cfg.ynab_config, token),
            config::Provider::Revolut(token) => revolut::initialize(token),
        };
        if refreshed {
            config::save_config(&cfg.path, cfg)?;
        }

        match connection {
            Ok(connection) => {
                println!("Connected: {:#?}", connection);
                connected.push(connection);
            }
            Err(e) => eprintln!("Couldn't connect to {:#?}\n{}", &mut cfg.providers[idx], e),
        }
    }

    Ok(connected)
}
