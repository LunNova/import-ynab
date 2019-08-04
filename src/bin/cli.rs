use import_ynab::cli;
use import_ynab::prelude::*;

fn main() -> Result<(), Error> {
    if std::env::var_os("RUST_LOG") == None {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    cli::run()
}
