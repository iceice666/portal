use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use inquire::Select;
use portal_core::{
    broadcast::{Listener, Sender},
    master::{Master, TaskStatus},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::error::{CrateResult, Error};

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
    EditConfig: "Edit the configuration" ,
    ScanDevices: "Scan for devices" ,
    ListDevices: "List all devices" ,
    MakeAvailable: "Make this device detectable" ,
    SetTarget: "Set the target device"
    SendFile: "Send a file to a device" ,
    PauseTask: "Pause a task" ,
    ResumeTask: "Resume a task" ,
    AbortTask: "Abort a task" ,
    ListTask: "List all tasks" ,
);

#[derive(Debug)]
pub(crate) struct Manager {
    broadcast_sender: Arc<Sender>,
    broadcast_listener: Listener,
    master: Option<Master>,
    slaves: Vec<Master>,
    task: HashMap<String, (UnboundedSender<TaskStatus>, UnboundedReceiver<CrateResult>)>,
}

impl Manager {
    pub fn try_new(
        service_port: u16,
        broadcast_port: u16,
        listening_port: u16,
    ) -> CrateResult<Self> {
        let broadcast_sender = Arc::new(Sender::new(service_port, broadcast_port)?);
        let broadcast_listener = Listener::new(listening_port)?;

        Ok(Self {
            broadcast_listener,
            broadcast_sender,
            master: None,
            slaves: Vec::new(),
            task: HashMap::new(),
        })
    }

    pub(crate) fn refresh(
        &mut self,
        service_port: u16,
        broadcast_port: u16,
        listening_port: u16,
    ) -> CrateResult<()> {
        let _ = std::mem::replace(
            self,
            Self::try_new(service_port, broadcast_port, listening_port)?,
        );

        Ok(())
    }

    #[async_recursion::async_recursion]
    pub async fn dispatch(&'static self, cmd: MainCommand) -> CrateResult {
        match cmd {
            MainCommand::Exit => Err(Error::Exit),
            MainCommand::EditConfig => {
                todo!();
            }

            MainCommand::ScanDevices => {
                println!("Scanning devices in 30 secs...");

                self.broadcast_listener
                    .async_scan_device(Duration::from_secs(30));

                println!("Scanning complete.");

                Ok(())
            }

            MainCommand::ListDevices => {
                let devices = &self.broadcast_listener.scanned_devices;

                if devices.is_empty() {
                    println!("No devices found.");
                } else {
                    println!("Devices found:");
                    for device in devices {
                        println!("{}", device);
                    }
                }

                Ok(())
            }

            MainCommand::MakeAvailable => {
                let sender = Arc::clone(&self.broadcast_sender);

                // let _ = tokio::time::timeout(
                //     Duration::from_secs(60),
                //     sender.async_send_loop(Duration::from_secs(1)),
                // )
                // .await;
                tokio::spawn(async move {
                    let _ = tokio::time::timeout(
                        Duration::from_secs(60),
                        sender.async_send_loop(Duration::from_secs(1)),
                    )
                    .await;
                });

                println!("This device is now detectable by other devices in 60 secs.");

                Ok(())
            }

            MainCommand::SendFile => {
                let path = PathBuf::from("Cargo.toml");

                if self.master.is_none() {
                    println!("You have to set target first!");
                } else {
                    let _ = self.master.as_mut().unwrap().send_a_file(path).await;
                }
                Ok(())
            }
            MainCommand::SetTarget => {
                let devices = &self.broadcast_listener.scanned_devices;

                if devices.is_empty() {
                    println!("No devices found.");
                    return Ok(());
                }

                let ans = Select::new(
                    "Select a device to send the file to",
                    devices.iter().collect(),
                )
                .prompt()
                .map_err(|_| Error::InvalidInput)?;

                // self.master = Some(TcpStream::connect(ans)?.try_into()?);

                Ok(())
            }
            MainCommand::PauseTask => {
                todo!();
            }
            MainCommand::ResumeTask => {
                todo!();
            }
            MainCommand::AbortTask => {
                todo!();
            }
            MainCommand::ListTask => {
                todo!();
            }
        }
    }
}
