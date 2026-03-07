mod build;
mod history;
mod init;
mod rollback;
mod status;
mod switch;
mod update;

pub use build::execute as build;
pub use history::execute as history;
pub use init::execute as init;
pub use rollback::execute as rollback;
pub use status::execute as status;
pub use switch::execute as switch;
pub use update::execute as update;
