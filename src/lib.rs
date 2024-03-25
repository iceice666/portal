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
//!               || ==========================> || Allow this connection
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
//!               || ==========================> || Merge temp data
//!               ||                             ||
//!               || Send metadata               ||
//!               || ==========================> || Save file

mod broadcast;
mod frame;
mod side;
mod utils;
