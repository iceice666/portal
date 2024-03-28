use crate::command::MainCommand;
use error::Error;
use inquire::Select;
use portal_core::master::{Master, MasterConfig};

type AnyResult<T = ()> = anyhow::Result<T>;

mod command;
mod error;

#[tokio::main]
async fn main() -> AnyResult {
    env_logger::init();

    let mut master = Master::new(MasterConfig::default())?;

    loop {
        println!();

        let ans = Select::new(
            "Greetings! What would you like to do today?",
            MainCommand::get_options(),
        )
        .prompt();

        if ans.is_err() {
            println!("Invalid input. Please try again.");
            continue;
        }

        let main_ans = ans.unwrap();

        let cmd = MainCommand::new(main_ans);
        if cmd.is_none() {
            println!("Invalid input. Please try again.");
            continue;
        }
        let cmd = cmd.unwrap();
        let res = cmd.dispatch(&mut master).await;

        if res.is_ok() {
            continue;
        }

        let error = res.unwrap_err();
        error.handle();

        if matches!(error, Error::Exit) {
            break;
        };
    }

    Ok(())
}
