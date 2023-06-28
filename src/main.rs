mod gpio_facade;
mod output_facade;
use std::{fs,path::Path,io::{stdin,stdout}, thread,time::Duration,sync::Arc};
use chrono::{DateTime,Local};
use gpio_facade::{Fixture,Direction};

use crate::output_facade::{TestState, OutputFile};

const VERSION:&str = "0.0.0-alpha.1";
const DEFAULT_ITERATIONS:u64 = 10;
const CAMERA_FILE_PREFIX:&str = "video-cam-";

fn main() {
    setup_logs();
    log::info!("Rust OCR version {}",VERSION);
    let mut serials_set = false;
    let mut cameras_configured = false;
    let mut iteration_count = DEFAULT_ITERATIONS;

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

    match fs::read_dir("/dev"){
        Ok(dev_directory) =>{
            //set up serial connections, with priority given to symlinks based on location, then
            //serial, then raw devices.
            todo!();
        },
        Err(error) =>{
            log::warn!("Could not open dev directory! Did you run with `sudo`?");
            log::debug!("{}",error);
        }
    }

    loop{
        //Send fixture down
        //for all serials:
        //  fire single device temp measurement
        //  request serial from user
        //  save serial to device
        //
        //request iteration count from user (if not set in CLI)
        //
        //for iteration count:
        //  fixture up
        //  fixture down
        //  new thread for each device:
        //      fire temp measurement
        //      parse response
        //      save value to appropriate output file as discrete frequency distribution table
        //  wait till all threads return


        //OLD MAIN
        //------------
        //print_main_menu(iteration_count,cameras_configured,serials_set);

        //match get_user_number_input(){
        //    1 => {
        //        configure_cameras();
        //        cameras_configured = true;
        //    },
        //    2 => {
        //        configure_serials();
        //        serials_set = true;
        //    },
        //    3 => iteration_count = set_iteration_count(),
        //    4 => set_active_cameras(),
        //    5 => run_tests(&mut fixture,&mut available_cameras,cameras_configured,iteration_count),
        //    6 => print_main_menu_help(),
        //    7 => break,
        //    _ => log::warn!("Invalid user input! Please input a valid number.")
        //}
    }
}

fn get_user_number_input() -> u64{
    let mut user_input:String = String::default();
    match stdin().read_line(&mut user_input){
        Ok(_) => {
            match user_input.trim().parse(){
                Ok(value)    => return value,
                Err(error)=>{
                    log::warn!("User input cannot be parsed!");
                    log::debug!("{}",error);
                }
            }
        },
        Err(error) => {
            log::warn!("Unable to read user input!");
            log::debug!("{}",error);
        }
    }
    return 0;
}

fn setup_logs() {
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
        .chain(
            fern::Dispatch::new()
                .level(log::LevelFilter::Trace)
                .chain(fern::log_file(format!("logs/{}.log",
                    chrono_now.format("%Y-%m-%d_%H.%M").to_string())).unwrap()
                )
        )
        .chain(
            fern::Dispatch::new()
                .level(log::LevelFilter::Info)
                .chain(stdout())
        )
        .apply();
}
