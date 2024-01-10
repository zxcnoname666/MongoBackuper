mod backuper;
mod logger;
mod exts;

#[cfg(not(target_os = "windows"))]
const DIRECTORY: &'static str = "/MongoBackups";

#[cfg(target_os = "windows")]
const DIRECTORY: &'static str = "C:\\MongoBackups";


#[cfg(not(target_os = "windows"))]
#[path = "core/linux.rs"]
mod core;

#[cfg(target_os = "windows")]
#[path = "core/windows.rs"]
mod core;

#[cfg(target_os = "windows")]
#[macro_use]
extern crate windows_service;


fn main() {
    core::main();
}