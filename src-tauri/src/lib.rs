#[cfg(feature = "desktop")]
pub mod commands;
pub mod core;
pub mod db;
pub mod models;
pub mod parsers;
pub mod scope;
pub mod services;
pub mod vault;

pub mod domain;
pub mod errors;
pub mod identity;
