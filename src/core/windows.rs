const SERVICENAME: &'static str = "mongo_backuper";

use {
    tokio::time::Duration,
    std::{path::Path, fs, process, ffi::OsString},
    windows_service::service_dispatcher,
    windows_service::service_control_handler::{self, ServiceControlHandlerResult},
    windows_service::service_manager::{ServiceManager, ServiceManagerAccess},
    windows_service::service::{
        ServiceControl, ServiceInfo, ServiceType, 
        ServiceStartType, ServiceAccess, ServiceErrorControl, 
        ServiceState, ServiceStatus, ServiceControlAccept, ServiceExitCode
    }
};

define_windows_service!(ffi_service_main, service_init);

fn service_init(arguments: Vec<OsString>) {
    if let Err(err) = service_run(arguments) {
        crate::logger::error_string(format!("Service Init: {err}"));
    }
}

fn service_run(_: Vec<OsString>) -> Result<(), windows_service::Error> {

    let event_handler = move | control_event | -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                process::exit(0x0000);
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICENAME, event_handler)?;

    let proccess_id: Option<u32> = Some(process::id());
    let service_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: proccess_id,
    };
    
    status_handle.set_service_status(service_status)?;

    crate::backuper::run();
    
    Ok(())
}

pub fn main() {
    let mut run_dir = format!("{}", std::env::var("USERPROFILE").unwrap_or_default());
    run_dir.remove(0);

    if run_dir.starts_with(":\\WINDOWS\\system32") {
        if let Err(err) = service_dispatcher::start(SERVICENAME, ffi_service_main) {
            println!("Error: {:?}", err);
        }
    } else {
        {
            let args: Vec<String> = std::env::args().collect();
        
            if args.len() > 1 {
                process_command(args[1].to_lowercase().as_str());
            }
        }

        loop {
            println!("{} Write command... // Write \"help\" to get commands", crate::logger::colors::green("{INPUT}"));

            let readed = crate::exts::read_line();
            process_command(readed.as_str());
        }
    }
}

fn process_command(command: &str) {
    match command {

        "help" => {
            crate::logger::info("Command list:");
            crate::logger::info("| help - Get a list of commands");
            crate::logger::info("| install - Install a service for automatic backups");
            crate::logger::info("| uninstall - Remove a service for automatic backups");
            crate::logger::info("| restart - Restart a service for automatic backups");
            crate::logger::info("| run - Run the backup script");
            crate::logger::info("| quit - Close the app");
        }

        "install" => {
            let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
            let service_manager = match ServiceManager::local_computer(None::<&str>, manager_access) {
                Ok(res) => res,
                Err(err) => {
                    crate::logger::warn_string(format!("Failed to create a ServiceManager session: {err}"));
                    return;
                }
            };

            let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
            if let Ok(service) = service_manager.open_service(SERVICENAME, service_access) {
                if let Err(err) = service.delete() {
                    crate::logger::warn_string(format!("Failed to delete old service: {err}"));
                }
                match service.query_status() {
                    Ok(status) => {
                        if status.current_state != ServiceState::Stopped {
                            if let Err(err) = service.stop() {
                                crate::logger::warn_string(format!("Failed to stop old service: {err}"));
                            }
                        }
                    },
                    Err(err) => {
                        crate::logger::warn_string(format!("Failed to get current status of old service: {err}"));
                    }
                }
            }

            
            let service_file_path = Path::new("C:\\ProgramData\\MongoBackuper");

            if service_file_path.exists() {
                if let Err(err) = fs::remove_dir_all(service_file_path) {
                    crate::logger::warn_string(format!("Error when deleting a exists directory: {err}"));
                }
            }

            if let Err(err) = fs::create_dir_all(service_file_path) {
                crate::logger::warn_string(format!("Error when creating a directory: {err}"));
                return;
            }

            let current_path = match std::env::current_exe() {
                Ok(path) => path,
                Err(err) => {
                    crate::logger::warn_string(format!("Error when getting the location of the current file: {err}"));
                    return;
                }
            };
            
            let exec_file_path = service_file_path.join("MongoBackuper.exe");
            if let Err(err) = fs::copy(current_path, &exec_file_path) {
                crate::logger::warn_string(format!("Error when copying a file: {err}"));
                return;
            }

            
            let service_info = ServiceInfo {
                name: OsString::from(SERVICENAME),
                display_name: OsString::from("MongoBackuper"),
                service_type: ServiceType::OWN_PROCESS,
                start_type: ServiceStartType::AutoStart,
                error_control: ServiceErrorControl::Normal,
                executable_path: exec_file_path,
                launch_arguments: vec![],
                dependencies: vec![],
                account_name: None,
                account_password: None,
            };

            let service_open_access = ServiceAccess::CHANGE_CONFIG | ServiceAccess::START;
            let service = match service_manager.create_service(&service_info, service_open_access) {
                Ok(res) => res,
                Err(err) => {
                    crate::logger::warn_string(format!("Failed to create service: {err}"));
                    return;
                }
            };

            let args: [OsString; 0] = [];
            if let Err(err) = service.start(&args) {
                crate::logger::warn_string(format!("Failed to start service: {err}"));
            }

            if let Err(err) = service.set_description("Create backups of MongoDB") {
                crate::logger::warn_string(format!("Failed to change service desc: {err}"));
            }

            crate::logger::info("Service created");
        }

        "uninstall" => {
            let manager_access = ServiceManagerAccess::CONNECT;
            let service_manager = match ServiceManager::local_computer(None::<&str>, manager_access) {
                Ok(res) => res,
                Err(err) => {
                    crate::logger::warn_string(format!("Failed to create a ServiceManager session: {err}"));
                    return;
                }
            };

            let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
            if let Ok(service) = service_manager.open_service(SERVICENAME, service_access) {
                if let Err(err) = service.delete() {
                    crate::logger::warn_string(format!("Failed to delete service: {err}"));
                }
                match service.query_status() {
                    Ok(status) => {
                        if status.current_state != ServiceState::Stopped {
                            if let Err(err) = service.stop() {
                                crate::logger::warn_string(format!("Failed to stop service: {err}"));
                            }
                        }
                    },
                    Err(err) => {
                        crate::logger::warn_string(format!("Failed to get current status of service: {err}"));
                    }
                }
            }

            let service_file_path = Path::new("C:\\ProgramData\\MongoBackuper");
            
            if service_file_path.exists() {
                if let Err(err) = fs::remove_dir_all(service_file_path) {
                    crate::logger::warn_string(format!("Error when deleting a exists directory: {err}"));
                }
            }

            crate::logger::info("Service deleted");
        }

        "restart" => {
            let manager_access = ServiceManagerAccess::CONNECT;
            let service_manager = match ServiceManager::local_computer(None::<&str>, manager_access) {
                Ok(res) => res,
                Err(err) => {
                    crate::logger::warn_string(format!("Failed to create a ServiceManager session: {err}"));
                    return;
                }
            };

            let service_access = ServiceAccess::STOP | ServiceAccess::START;
            if let Ok(service) = service_manager.open_service(SERVICENAME, service_access) {
                if let Err(err) = service.stop() {
                    crate::logger::warn_string(format!("Failed to stop service: {err}"));
                }

                let args: [OsString; 0] = [];
                if let Err(err) = service.start(&args) {
                    crate::logger::warn_string(format!("Failed to start service: {err}"));
                }
            }

            crate::logger::info("Service restarted");
        }

        "run" => {
            crate::backuper::run();
        }

        "quit" => {
            crate::exts::close_proc();
        }

        _ => {
            crate::logger::warn_string(format!("Unknow command: {command}"));
        }
    }
}