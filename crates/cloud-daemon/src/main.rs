use cloud_core::{
    CloudCore,
    logger::logger::{
        LogLevel::{Error, Info},
        log,
    },
};

fn main() {
    log(Info, "Starting BuliCloud daemon ...");
    let cloud_core = CloudCore::new("./bulicloud");

    match cloud_core.initialize() {
        Ok(_) => {
            log(Info, "BuliCloud-Core initialized successfully");
        }
        Err(error) => {
            log(Error, &format!("Failed to initialized BuliCloud-Core: {}", error));
            return;
        }
    }
}
