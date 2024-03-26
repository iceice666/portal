//! How to we transfer data?
//!
//!             Master                          Slave
//! Which the data originates from -> Which the data is sent to
//!               ||                             ||
//!               ||                             ||
//!               ||              Udp broadcast  ||
//!    Fetched    || <========================== ||
//!   ip and port ||                             ||
//!               ||                             ||
//!               || Tcp connection              ||
//!               || ==========================> || connected!
//!               ||                             ||
//!               ||                             ||
//!               || Sending data ...            ||
//!               || ==========================> || Save temp data
//!               || ==========================> ||
//!               || ==========================> ||
//!               || ==========================> ||
//!               ||                             ||
//!               ||                             ||
//!               || End signal                  ||
//!               || ==========================> || Merge data
//!               ||                             ||

mod broadcast;
pub mod error;
pub mod master;
mod side;
mod utils;
