#![no_std]

pub mod contract;
mod dependencies;
mod errors;
mod events;
mod storage;
pub mod types;

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod testutils;
