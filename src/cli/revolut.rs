use crate::prelude::*;

use crate::revolut::auth::*;
use crate::revolut::*;

pub fn handle(args: RevolutClient) -> Result<(), Error> {
    let c = Client::new(new_rest_client(&args.device_id));

    match args.commands {
        RevolutClientCommands::Auth(args) => auth(c, args),
        RevolutClientCommands::Transactions(args) => transactions(c, args),
    }

    Ok(())
}

pub fn auth(mut c: Client, args: AuthCommands) {
    match args {
        AuthCommands::RequestSignin(sr) => c.signin(&sr),
        AuthCommands::ConfirmSignin(csr) => c.confirm_signin(&csr),
    }
}

pub fn transactions(mut c: Client, args: TransactionArgs) {
    //c.auth(&args.auth);

    match args.command {
        TransactionCommands::Export => println!("{:#?}", c.get_transactions()),
    }
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
pub struct RevolutClient {
    #[structopt(long = "device-id", env = "REVOLUT_DEVICE_ID")]
    device_id: String,
    #[structopt(subcommand)]
    commands: RevolutClientCommands,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum RevolutClientCommands {
    Auth(AuthCommands),
    Transactions(TransactionArgs),
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum AuthCommands {
    RequestSignin(SigninRequest),
    ConfirmSignin(ConfirmSigninRequest),
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct TransactionArgs {
    //    #[structopt(flatten)]
    //    auth: Auth,
    #[structopt(subcommand)]
    command: TransactionCommands,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum TransactionCommands {
    Export,
}
