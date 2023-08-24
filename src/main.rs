mod gpio_facade;
mod output_facade;
mod serial;
use std::{fs,path::Path,io::{Write,stdin,stdout}, thread::{self, JoinHandle}, sync::{Arc, atomic::AtomicBool}};
use chrono::{DateTime,Local};
use gpio_facade::{Fixture,Direction};
use glob::glob;
use signal_hook;
use clap::Parser;
use crate::{serial::TTY, output_facade::{OutputFile, TestState}};


const VERSION:&str = "5.0.0";
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
    //Listen for kernel-level signals to exit. These are sent by keyboard shortcuts:
    let terminate = Arc::new(AtomicBool::new(false));
    //There is not a keyboard shortcut for SIGTERM, generally sent by a task manager like htop
    //or btop
    _ = signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&terminate));
    //SIGINT = Ctrl+c
    _ = signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&terminate));
    //SIGQUIT = Ctrl+\
    _ = signal_hook::flag::register(signal_hook::consts::SIGQUIT, Arc::clone(&terminate));

    //Import command-line arguments
    let args = Args::parse();

    setup_logs(&args.debug);

    //Repot version of software to user and log file
    log::info!("Rust OCR version {}",VERSION);

    //Initialise fixture
    let mut fixture:Option<Fixture> = None;
    //Keep trying until the fixture inits properly, or the user overrides
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

    //As long as the user doesn't kill the process, continue to loop here
    while !terminate.load(std::sync::atomic::Ordering::Relaxed)
    {

        log::info!("Finding devices connected to debug cables....");
        let mut available_ttys:Vec<Box<Path>> = Vec::new();

        //Try to detect serial devices by proper symlinks
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
        //If no symlinks exist, also check USB serial devices
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

        //We're talking to the disco over serial; if we can't open a serial connection, we can't
        //talk to the device. End the program here.
        if available_ttys.is_empty(){
            log::error!("No serial devices detected! Please ensure all connections.");
            return;
        }

        //We now have a list of possible TTY device locations. Try to open them, each in their
        //own thread
        let mut possible_devices: Vec<Option<TTY>> = Vec::new();
        let mut tty_test_threads: Vec<JoinHandle<Option<TTY>>> = Vec::new();
        for tty in available_ttys.into_iter(){
            tty_test_threads.push( thread::spawn( move|| {
                match TTY::new(&tty.to_string_lossy()){
                    Some(port) => {
                        log::info!("Found device {}!",port.get_serial());
                        Some(port)
                    }
                    None => None
                }
            }));
        }

        //Get the possible TTYs from the above threads
        for thread in tty_test_threads{
            let output = thread.join().unwrap_or_else(|x|{log::trace!("{:?}",x); None});
            possible_devices.push(output);
        }

        //Filter possible TTYs down to real ones; check that their serials are set
        let mut serials_set:bool = true;
        let mut devices:Vec<TTY> = Vec::new();
        let mut device_names:Vec<String> = Vec::new();
        for possible_device in possible_devices.into_iter(){
            if let Some(device) = possible_device{
                if device.get_serial().eq("unknown"){
                    serials_set = false;
                }
                device_names.push(device.get_serial().to_string());
                devices.push(device);
            }
        }

        //Create a new output file and storage of the current state of the test
        let mut out_file: OutputFile = OutputFile::new(device_names.clone());
        let state: TestState = TestState::new(device_names);

        //Tell the user how many devices we have
        log::info!("--------------------------------------");
        log::info!("Number of devices detected: {}",devices.len());
        log::info!("--------------------------------------\n\n");

        //If we need to set the serials manually for the device....
        //well this will require a udev rule. 
        //Check https://git.blizzard.systems/blizzardfinnegan/javaOCR for more information on
        //udev rules.
        for _device in devices.iter_mut(){
            if !serials_set || args.manual{
              todo!();
            }
        }

        //If the user set an iteration count in the CLI, then just use that, don't prompt
        let iteration_count:u64;
        if let Some(count) = args.iterations{
            iteration_count = count;
        }
        //If the user didn't set an iteration count, prommpt for it
        else{
            print!("How many times would you like to test the devices attached to the fixture?  Enter '0' to quit: \n> ");
            _ = stdout().flush();
            let mut user_input:String = String::new();
            stdin().read_line(&mut user_input).expect("Did not input a valid number.");
            //Minor string parsing/cleanup
            //********
            if let Some('\n') = user_input.chars().next_back() {
                user_input.pop();
            };
            if let Some('\r') = user_input.chars().next_back() {
                user_input.pop();
            };
            //********

            //Convert the string to an integer; fallback to default iteration count of 10
            match user_input.parse::<u64>(){
                Err(_) => {
                    iteration_count = DEFAULT_ITERATIONS; 
                },
                Ok(parsed_user_input) => {
                    if parsed_user_input == 0 { break; }
                    else { iteration_count = parsed_user_input; }
                }
            }
        }

        //Assuming we haven't gotten a kill signal yet from the kernel, keep going
        if !terminate.load(std::sync::atomic::Ordering::Relaxed){
            for iter in 0..iteration_count{
                log::info!("Starting iteration {} of {}...",iter+1, iteration_count);
                if let Some(ref mut real_fixture) = fixture{
                    if terminate.load(std::sync::atomic::Ordering::Relaxed) { break; }
                    real_fixture.goto_limit(Direction::Up);
                    if terminate.load(std::sync::atomic::Ordering::Relaxed) { break; }
                    real_fixture.goto_limit(Direction::Down);
                    if terminate.load(std::sync::atomic::Ordering::Relaxed) { break; }
                    real_fixture.push_button();
                    if terminate.load(std::sync::atomic::Ordering::Relaxed) { break; }
                }
                //Get the temperature from the device; if its a bad value, default to f32::MAX
                //Save out to file
                for ref mut device in devices.iter_mut(){
                    state.add_iteration(device.get_serial().to_string(), device.get_temp().unwrap_or(f32::MAX));
                }
                out_file.write_values(&state, None, None);

                //Check again for the kill signal from kernel
                if terminate.load(std::sync::atomic::Ordering::Relaxed) { break; }
            }
        }
    }
    //Before exiting, reset the fixture arm
    if let Some(ref mut real_fixture) = fixture{
        real_fixture.goto_limit(Direction::Up);
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
            //Write verbose logs to log file
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
            //Use higher level logging as wrapper for print to user
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
