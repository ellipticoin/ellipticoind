pub mod db;
pub mod traits;

pub use db::DB;
pub const ADDRESS_LENGTH: usize = 20;
pub type Address = [u8; ADDRESS_LENGTH];
