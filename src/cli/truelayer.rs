use crate::prelude::*;

#[derive(StructOpt)]
pub struct TruelayerArgs {
    #[structopt(subcommand)]
    command: TruelayerCommands,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum TruelayerCommands {
    AuthUrl,
    Auth(AuthArgs),
}

#[derive(StructOpt)]
pub struct AuthArgs {
    token: String,
}

#[derive(StructOpt)]
pub struct TruelayerCredentials {}

pub fn handle(args: TruelayerArgs) -> Result<(), Error> {
    match args.command {
        TruelayerCommands::AuthUrl => println!("{}", crate::truelayer::get_auth_url()?),
        TruelayerCommands::Auth(args) => println!(
            "{}",
            serde_json::to_string_pretty(&crate::truelayer::authorize(args.token)?)?
        ),
    }

    Ok(())
}
