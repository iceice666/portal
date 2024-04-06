use crate::command::MainCommand;
use command::Manager;
use error::Error;
use inquire::Select;

type AnyResult<T = ()> = anyhow::Result<T>;

mod command;
mod error;

#[tokio::main]
async fn main() -> AnyResult {
    tracing_subscriber::fmt::init();

    let service_port = portpicker::pick_unused_port().expect("No ports available");
    let broadcast_port = portpicker::pick_unused_port().expect("No ports available");

    let mut manager = Manager::try_new(service_port, broadcast_port, broadcast_port)?;

    loop {
        // Print a newline to separate the previous output from the new one
        println!();

        // Prompt the user for input
        let ans = Select::new(
            "Greetings! What would you like to do today?",
            MainCommand::get_options(),
        )
        .prompt();

        // If the user input is invalid, prompt them again
        let main_ans = match ans {
            Err(_) => {
                println!("Invalid input. Please try again.");
                continue;
            }
            Ok(ans) => ans,
        };

        // If this command is invalid, prompt the user again
        let cmd = match MainCommand::new(main_ans) {
            Some(cmd) => cmd,
            None => {
                println!("Invalid input. Please try again.");
                continue;
            }
        };

        match manager.dispatch(cmd).await {
            Ok(_) => {}
            Err(Error::Exit) => break,
            Err(e) => e.handle(),
        }
    }

    Ok(())
}
