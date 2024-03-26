use crate::error::Error;

pub trait Command {
    fn new(input: &str) -> Option<Self>
    where
        Self: Sized;

    fn get_options() -> Vec<&'static str>
    where
        Self: Sized;

    fn dispatch(self) -> Result<(), Error>
    where
        Self: Sized;
}

macro_rules! cmd_opt {
    (
        $name:ident,
        $(
            $member:ident: $desc:literal
            $(=>)? $exec:expr
            $(,)?
        )*
    ) => {
        #[derive(Debug, PartialEq, Eq)]
        pub enum $name {
            $(
                $member,
            )*
        }

        impl Command for $name {
            fn new(input: &str) -> Option<Self> {
                match input {
                    $(
                        $desc => Some(Self::$member),
                    )*
                    _ => None,
                }
            }

            fn get_options() -> Vec<&'static str> {
                vec![
                    $(
                        $desc,
                    )*
                ]
            }

            fn dispatch(self) -> Result<(), Error> {
                match self {
                    $(
                        Self::$member => $exec,
                    )*
                }
            }
        }


    };
}

cmd_opt!(
    MainCommand,
    Exit: "Exit the program" => Err(Error::Exit),
    Config: "Edit the configuration" {
                todo!()
    },
    ScanDevices: "Scan for devices" {
                todo!()
    },
    ListDevices: "List all devices" {
                todo!()
    },
    SendFile: "Send a file to a device" {
                todo!()
    },
    RecvFile: "Receive a file from a device" {
                todo!()
    },
    PauseTask: "Pause a task" {
                todo!()
    },
    ResumeTask: "Resume a task" {
                todo!()
    },
    AbortTask: "Abort a task" {
                todo!()
    },
    ListTask: "List all tasks" {
                todo!()
    },
);
