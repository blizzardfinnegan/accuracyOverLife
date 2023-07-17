mod gpio_facade;
mod output_facade;
mod serial;
use std::{fs,path::Path,io::{Write,stdin,stdout}, thread::{self, JoinHandle}};
use chrono::{DateTime,Local};
use gpio_facade::{Fixture,Direction};
use glob::glob;
use clap::Parser;
use crate::{serial::TTY, output_facade::{OutputFile, TestState}};


const VERSION:&str = "5.0.0-alpha.1";
const DEFAULT_ITERATIONS:u64 = 10;


#[derive(Parser,Debug)]
#[command(author,version,about)]
struct Args{
    /// Print all logs to screen, improves log verbosity. Sets iteration count to 50000
    #[arg(short,long,action)]
    debug:bool,

    /// Force manually setting serial numbers
    #[arg(short,long,action)]
    manual:bool,

    /// Set iteration count from command line. Overrides debug iteration count.
    #[arg(short,long)]
    iterations:Option<u64>

}
fn main() {
    let args = Args::parse();
    setup_logs(&args.debug);
    log::info!("Rust OCR version {}",VERSION);

    //Initialise fixture
    let mut fixture:Option<Fixture> = None;
    loop {
        match Fixture::new() {
            Ok(fixture_object) => { 
                fixture = Some(fixture_object);
                break; 
            },
            _ => {
                print!("Fixture initialisation failed! Press enter to try again.");
                let mut user_input = String::new();
                stdin().read_line(&mut user_input).expect("Failed user input");
                let clean_input = user_input.trim();
                if clean_input.contains("override"){
                    break;
                }
            }
        }
    }

    #[allow(unreachable_code)]
    loop{
        let mut available_ttys:Vec<Box<Path>> = Vec::new();

        for entry in glob("/dev/serial/*").expect("Failed to read glob pattern"){
            match entry{
                Ok(real_path) =>{
                    match fs::read_dir::<&Path>(real_path.as_ref()){
                        Ok(possible_ttys) =>{
                            possible_ttys.into_iter().for_each(|tty| {
                                if let Ok(single_tty) = tty {
                                    available_ttys.push(single_tty.path().into());
                                }
                            });
                            break;
                        }
                        Err(error) =>{
                            log::error!("Invalid permissions to /dev directory... did you run with sudo?");
                            log::error!("{}",error);
                            return;
                        }
                    }
                }
                Err(error) =>{
                    log::error!("{}",error);
                }
            }
        }
        if available_ttys.is_empty(){
            for entry in glob::glob("/dev/ttyUSB*").expect("Unable to read glob"){
                match entry{
                    Ok(possible_tty) => available_ttys.push(Path::new(&possible_tty).into()),
                    Err(error) => {
                        log::error!("Invalid permissions to /dev directory... did you run with sudo?");
                        log::error!("{}",error);
                        return;
                    }
                };
            }
        }

        if available_ttys.is_empty(){
            log::error!("No serial devices detected! Please ensure all connections.");
            return;
        }

        if let Some(ref mut real_fixture) = fixture{
            real_fixture.goto_limit(Direction::Down);
            real_fixture.push_button();
        }

        let mut possible_devices: Vec<Option<TTY>> = Vec::new();
        let mut tty_test_threads: Vec<JoinHandle<Option<TTY>>> = Vec::new();
        for tty in available_ttys.into_iter(){
            tty_test_threads.push( thread::spawn( move|| {
                match TTY::new(&tty.to_string_lossy()){
                    Some(mut port) => {
                        log::info!("Found device {}!",port.get_serial());
                        Some(port)
                    }
                    None => None
                }
            }));
        }

        for thread in tty_test_threads{
            let output = thread.join().unwrap_or_else(|x|{log::trace!("{:?}",x); None});
            possible_devices.push(output);
        }
        let mut serials_set:bool = true;
        let mut devices:Vec<TTY> = Vec::new();
        let mut device_names:Vec<String> = Vec::new();
        for possible_device in possible_devices.into_iter(){
            if let Some(mut device) = possible_device{
                if device.get_serial().eq("unknown"){
                    serials_set = false;
                }
                device_names.push(device.get_serial().to_string());
                devices.push(device);
            }
        }

        let mut out_file: OutputFile = OutputFile::new(device_names.clone());
        let state: TestState = TestState::new(device_names);

        log::info!("--------------------------------------");
        log::info!("Number of devices detected: {}",devices.len());
        log::info!("--------------------------------------\n\n");

        //for device in devices.iter_mut(){
        //    if !serials_set || args.manual{
        //      todo!()
        //    }
        //}

        print!("How many times would you like to test the devices attached to the fixture?");
        _ = stdout().flush();
        let mut user_input:String = String::new();
        stdin().read_line(&mut user_input).expect("Did not input a valid number.");
        if let Some('\n') = user_input.chars().next_back() {
            user_input.pop();
        };
        if let Some('\r') = user_input.chars().next_back() {
            user_input.pop();
        };

        let iteration_count:u64 = user_input.parse().unwrap_or(DEFAULT_ITERATIONS);

        for iter in 0..iteration_count{
            log::info!("Starting iteration {} of {}...",iter, iteration_count);
            if let Some(ref mut real_fixture) = fixture{
                real_fixture.goto_limit(Direction::Up);
                real_fixture.goto_limit(Direction::Down);
                real_fixture.push_button();
                for ref mut device in devices.iter_mut(){
                    state.add_iteration(device.get_serial().to_string(), device.get_temp().unwrap_or(f32::MAX));
                }
                out_file.write_values(&state, None, None);
            }
        }
    }
}

fn setup_logs(debug:&bool) {
    let chrono_now:DateTime<Local> = Local::now();
    if !Path::new("logs").is_dir(){ _ = fs::create_dir("logs"); }
    _ = fern::Dispatch::new()
        .format(|out,message,record|{
            out.finish(format_args!(
                "{}  [{}, {}] - {}",
                Local::now().to_rfc3339(),
                record.level(),
                record.target(),
                message
            ))
        })
        .chain({
            let mut file_logger = fern::Dispatch::new();
            let date_format = chrono_now.format("%Y-%m-%d_%H.%M").to_string();
            let local_log_file = fern::log_file(format!("logs/{}.log",date_format)).unwrap();
            if *debug{
                file_logger = file_logger.level(log::LevelFilter::Trace);
            }
            else {
                file_logger = file_logger.level(log::LevelFilter::Debug);
            }
            file_logger.chain(local_log_file)
        })
        .chain({
            let mut stdout_logger = fern::Dispatch::new();
            if *debug {
                stdout_logger = stdout_logger.level(log::LevelFilter::Trace);
            }
            else {
                stdout_logger = stdout_logger.level(log::LevelFilter::Info);
            }
                stdout_logger.chain(std::io::stdout())
        })
        .apply();
}
