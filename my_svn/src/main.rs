mod my_error;
mod repository;
mod restore;
mod run;
mod structures;
mod test;

fn main() {
    #[cfg(feature = "run")]
    {
        run::run();
    }

    #[cfg(feature = "restore")]
    {
        use colored::Colorize;
        if let Err(e) = restore::restore_svn() {
            println!("{}", format!("Error restore svn: {e}").bold().red());
        }
    }
}
