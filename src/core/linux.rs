pub fn main() {
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

fn process_command(command: &str) {
    match command {
        "help" => {
            crate::logger::info("Command list:");
            crate::logger::info("| help - Get a list of commands");
            crate::logger::info("| run - Run the backup script");
            crate::logger::info("| quit - Close the app");
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