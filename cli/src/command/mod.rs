use std::time::Duration;

use portal_core::master::Master;
use tokio::time::timeout;

use crate::error::Error;

type AnyResult<T = ()> = anyhow::Result<T>;

macro_rules! cmd_opt {
    (
        $name:ident,
        $(
            $member:ident: $desc:literal
            $(,)?
        )*
    ) => {
        #[derive(Debug, PartialEq, Eq)]
        pub enum $name {
            $(
                $member,
            )*
        }

        impl $name {
            pub fn new(input: &str) -> Option<Self> {
                match input {
                    $(
                        $desc => Some(Self::$member),
                    )*
                    _ => None,
                }
            }

            pub fn get_options() -> Vec<&'static str> {
                vec![
                    $(
                        $desc,
                    )*
                ]
            }

        }


    };
}

cmd_opt!(
    MainCommand,
    Exit: "Exit the program",
    Config: "Edit the configuration" ,
    ScanDevices: "Scan for devices" ,
    ListDevices: "List all devices" ,
    MakeAvailable: "Make this device detectable by others" ,
    SendFile: "Send a file to a device" ,
    PauseTask: "Pause a task" ,
    ResumeTask: "Resume a task" ,
    AbortTask: "Abort a task" ,
    ListTask: "List all tasks" ,
);

impl MainCommand {
    #[async_recursion::async_recursion]
    pub async fn dispatch(self, master: &mut Master) -> Result<(), Error> {
        match self {
            Self::Exit => Err(Error::Exit),
            Self::Config => {
                todo!();
            }
            Self::ScanDevices => {
                master.scan_device()?;

                println!("Scanning complete.");

                Self::ListDevices.dispatch(master).await?;

                Ok(())
            }
            Self::ListDevices => {
                let res = timeout(Duration::from_secs(5), master.get_devices())
                    .await
                    .map_err(|_| Error::Timeout)?;

                if res.is_empty() {
                    println!("No devices found.");
                } else {
                    println!("Devices found:");
                    for device in res {
                        println!("{}", device);
                    }
                }

                Ok(())
            }
            Self::MakeAvailable => {
                todo!();
            }
            Self::SendFile => {
                todo!();
            }
            Self::PauseTask => {
                todo!();
            }
            Self::ResumeTask => {
                todo!();
            }
            Self::AbortTask => {
                todo!();
            }
            Self::ListTask => {
                todo!();
            }
        }
    }
}
