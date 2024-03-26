use crate::command::Command;
use crate::command::MainCommand;
use error::Error;
use inquire::Select;

type AnyResult<T = ()> = anyhow::Result<T>;

mod command;
mod error;

fn main() {
    loop {
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
        let res = cmd.dispatch();

        if res.is_ok() {
            continue;
        }

        let error = res.unwrap_err();
        error.handle();

        if error == Error::Exit {
            break;
        }
    }
}
