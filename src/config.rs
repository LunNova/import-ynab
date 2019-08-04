pub const DEFAULT_PATH: &str = "secrets/";

use crate::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub path: PathBuf,
    pub providers: Vec<Provider>,
    pub ynab_config: YnabConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YnabConfig {
    pub budget_id: String,
    pub access_token: String,
}

impl Default for YnabConfig {
    fn default() -> Self {
        YnabConfig {
            budget_id: "default".to_string(),
            access_token: "".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Provider {
    Truelayer(crate::truelayer::Token),
    Revolut(crate::revolut::Token),
}

pub fn load_config(path: impl Into<PathBuf>) -> Result<Config, Error> {
    let (path, providers, ynab) = config_paths(path);
    let providers = load_or_default(&providers)?;
    let ynab_config = load_or_default(&ynab)?;

    Ok(Config {
        path,
        providers,
        ynab_config,
    })
}

pub fn save_config(path: impl Into<PathBuf>, cfg: &Config) -> Result<(), Error> {
    let (_path, providers, ynab) = config_paths(path);

    make_backup(&providers)?;
    save_json(&providers, &cfg.providers)?;

    make_backup(&ynab)?;
    save_json(&ynab, &cfg.ynab_config)?;

    Ok(())
}

fn save_json<T>(path: &Path, t: &T) -> Result<(), Error>
where
    T: serde::Serialize,
{
    let mut file: File = File::create(path)?;
    serde_json::to_writer_pretty(&mut file, t)?;
    file.sync_all()?;

    Ok(())
}

fn load_or_default<'a, T>(path: &Path) -> Result<T, Error>
where
    for<'de> T: Deserialize<'de> + 'a,
    T: Default,
{
    if !path.exists() {
        println!("Couldn't load from {}", path.display());
        return Ok(Default::default());
    }

    Ok(serde_json::from_reader(File::open(path)?)?)
}

fn config_paths(path: impl Into<PathBuf>) -> (PathBuf, PathBuf, PathBuf) {
    let path: PathBuf = path.into();

    let mut dummy = path.clone();
    dummy.push("dummy");
    let providers = dummy.with_file_name("providers.json");
    let ynab = dummy.with_file_name("ynab.json");

    (path, providers, ynab)
}

fn make_backup(path: &Path) -> Result<(), Error> {
    let bak: PathBuf = path.with_extension("json.bak");

    if path.exists() {
        std::fs::copy(&path, &bak)?;
        File::open(bak)?.sync_all()?;
    }

    Ok(())
}

#[cfg(test)]
pub mod test {
    use crate::prelude::*;

    #[test]
    fn config() -> Result<(), Error> {
        let cfg = super::load_config(super::DEFAULT_PATH)?;
        super::save_config(&cfg.path, &cfg)?;

        Ok(())
    }
}
