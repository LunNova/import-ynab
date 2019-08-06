use import_ynab_lib::cli;
use import_ynab_lib::prelude::*;

fn main() -> Result<(), Error> {
    if std::env::var_os("RUST_LOG") == None {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    cli::run()
}
