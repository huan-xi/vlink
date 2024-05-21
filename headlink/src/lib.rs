pub mod api;
pub mod db;
pub mod network;
pub mod server;
pub mod client;

pub mod peer;

use once_cell::sync::Lazy;
use crate::db::snowflake::MySnowflakeGenerator;


pub static SNOWFLAKE: Lazy<MySnowflakeGenerator> = Lazy::new(MySnowflakeGenerator::default);