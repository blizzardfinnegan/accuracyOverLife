use std::time::Duration;
use async_std::sync::*;
use rppal::gpio::{Gpio,OutputPin,InputPin,Trigger,Level};
use std::thread;
use std::result::Result;
use futures::executor;
use std::fmt;

const POLL_DELAY:Duration = Duration::from_millis(10);
const MOTOR_ENABLE_ADDR:u8 = 22;
const MOTOR_DIRECTION_ADDR:u8 = 27;
const PISTON_ADDR:u8 = 25;
const RUN_SWITCH_ADDR:u8 = 10;
const UPPER_LIMIT_ADDR:u8 = 23;
const UPPER_NC_LIMIT_ADDR:u8 = 5;
const LOWER_LIMIT_ADDR:u8 = 24;
const LOWER_NC_LIMIT_ADDR:u8 = 6;

static MOVE_LOCK:RwLock<bool> = RwLock::new(true);

pub struct Fixture{
    gpio_api:Gpio,
    motor_direction:Option<OutputPin>,
    motor_enable: Option<OutputPin>,
    piston_enable: Option<OutputPin>,
    upper_limit: Option<InputPin>,
    upper_nc_limit: Option<InputPin>,
    lower_limit: Option<InputPin>,
    lower_nc_limit: Option<InputPin>,
}

pub enum Direction{Up,Down}

impl Drop for Fixture{
    fn drop(&mut self) {
        self.reset_arm();
    }
}

#[derive(Debug,Clone)]
pub struct FixtureInitError;

impl fmt::Display for FixtureInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid fixture PWM value")
    }
}

impl Fixture{
    //modify implementation to allow for multiple fixtures simultaneously
    pub fn new() -> Result<Self,FixtureInitError>{
        let possible_gpio = Gpio::new();
        if let Ok(gpio) = possible_gpio{
            let mut output = Self{
                gpio_api:gpio,
                motor_direction: None,
                motor_enable: None,
                piston_enable: None,
                upper_limit: None,
                upper_nc_limit:None,
                lower_limit: None,
                lower_nc_limit:None
            };

            match output.gpio_api.get(MOTOR_ENABLE_ADDR){
                Ok(pin) =>{
                    output.motor_enable = Some(pin.into_output_low());
                }
                Err(_) => {
                    log::error!("Motor enable pin unavailable!");
                    return Err(FixtureInitError)
                }
            }
            match output.gpio_api.get(MOTOR_DIRECTION_ADDR){
                Ok(pin) =>{
                    output.motor_direction = Some(pin.into_output_low());
                }
                Err(_) => {
                    log::error!("Motor direction pin unavailable!");
                    return Err(FixtureInitError)
                }
            }
            match output.gpio_api.get(PISTON_ADDR){
                Ok(pin) =>{
                    output.piston_enable = Some(pin.into_output_low());
                }
                Err(_) => {
                    log::error!("Motor direction pin unavailable!");
                    return Err(FixtureInitError)
                }
            }
            match output.gpio_api.get(UPPER_LIMIT_ADDR){
                Ok(pin) =>{
                    output.upper_limit = Some(pin.into_input_pulldown());
                }
                Err(_) => {
                    log::error!("Upper limit pin unavailable!");
                    return Err(FixtureInitError)
                }
            }
            match output.gpio_api.get(LOWER_LIMIT_ADDR){
                Ok(pin) =>{
                    output.lower_limit = Some(pin.into_input_pulldown());
                }
                Err(_) => {
                    log::error!("Lower limit pin unavailable!");
                    return Err(FixtureInitError)
                }
            }
            match output.gpio_api.get(UPPER_NC_LIMIT_ADDR){
                Ok(pin) =>{
                    output.upper_nc_limit = Some(pin.into_input_pulldown());
                }
                Err(_) => {
                    log::error!("Upper limit pin unavailable!");
                    return Err(FixtureInitError)
                }
            }
            match output.gpio_api.get(LOWER_NC_LIMIT_ADDR){
                Ok(pin) =>{
                    output.lower_nc_limit = Some(pin.into_input_pulldown());
                }
                Err(_) => {
                    log::error!("Lower limit pin unavailable!");
                    return Err(FixtureInitError)
                }
            }

            match output.gpio_api.get(RUN_SWITCH_ADDR){
                Ok(run_pin) =>{
                    _ = run_pin.into_input_pulldown().set_async_interrupt(Trigger::Both, |switch_state|{
                        let mut move_allowed = executor::block_on(MOVE_LOCK.write());
                        match switch_state {
                            Level::Low=> {
                                *move_allowed = false;
                            }
                            Level::High => {
                                *move_allowed = true;
                            }
                            
                        };
                    });
                },
                Err(_) => {
                    log::error!("Could not get run switch GPIO pin!");
                }
            }
            log::info!("GPIO initialised successfully! Finding fixture travel distance.");

            output.reset_arm();
            output.goto_limit(Direction::Down);
            output.goto_limit(Direction::Up);
            return Ok(output);

        }
        else { 
            log::error!("Gpio could not be opened! Did you run with 'sudo'? ");
            return Err(FixtureInitError) 
        }
    }

    fn reset_arm(&mut self) -> u16{
        log::debug!("Resetting arm...");
        if let (Some(upper_limit),Some(upper_nc_limit),          Some(motor_direction_pin),      Some(motor_enable_pin)) = 
               (self.upper_limit.as_mut(),self.upper_nc_limit.as_mut(), self.motor_direction.as_mut(), self.motor_enable.as_mut()){
            log::trace!("Upper limit: {}",upper_limit.is_high());
            log::trace!("Upper NC limit: {}",upper_nc_limit.is_high());
            if upper_limit.is_high(){
                {
                    while !*executor::block_on(MOVE_LOCK.read()) {log::trace!("blocking!");}
                    motor_direction_pin.set_low();//
                    motor_enable_pin.set_high()
                }
                thread::sleep(Duration::from_millis(500));
                motor_enable_pin.set_low()
            }
            while !*executor::block_on(MOVE_LOCK.read()){log::trace!("blocking!");}
            motor_direction_pin.set_high();//
            motor_enable_pin.set_high();
            let mut counter = 0;
            while upper_limit.is_low() && upper_nc_limit.is_high() {
                while !*executor::block_on(MOVE_LOCK.read()){
                    motor_enable_pin.set_low();
                }
                if *executor::block_on(MOVE_LOCK.read()) && motor_enable_pin.is_set_low(){
                    motor_enable_pin.set_high();
                }
                if upper_limit.is_high() && upper_nc_limit.is_low() { 
                    log::trace!("Breaking early!");
                    break; 
                }
                counter += 1;
                thread::sleep(POLL_DELAY);
            }
            motor_enable_pin.set_low();
            return counter;
        };
        return 0;
    }

    pub fn goto_limit(&mut self, direction:Direction) -> bool{
        let ref mut limit_sense:InputPin;
        let ref mut limit_nc_sense:InputPin;
        match direction{
            Direction::Down => {
                log::trace!("Sending fixture down...");
                if let (Some(obj),Some(obj2),Some(motor_direction_pin)) = (self.lower_limit.as_mut(),self.lower_nc_limit.as_mut(),self.motor_direction.as_mut()) {
                    limit_sense = obj;
                    limit_nc_sense = obj2;
                    motor_direction_pin.set_low();//
                }
                else { return false; }
            },
            Direction::Up => {
                log::trace!("Sending fixture up...");
                if let (Some(obj),Some(obj2),Some(motor_direction_pin)) = (self.upper_limit.as_mut(),self.upper_nc_limit.as_mut(),self.motor_direction.as_mut()) {
                    limit_sense = obj;
                    limit_nc_sense = obj2;
                    motor_direction_pin.set_high();//
                }
                else { return false; }
            }
        }

        if limit_sense.is_high() && limit_nc_sense.is_low(){ log::debug!("Fixture already at proper limit switch!"); return true; }

        if let Some(motor_enable_pin) = self.motor_enable.as_mut(){
            while !*executor::block_on(MOVE_LOCK.read()){}
            motor_enable_pin.set_high();
            thread::sleep(POLL_DELAY);
            while limit_sense.is_low() || limit_nc_sense.is_high(){
                while !*executor::block_on(MOVE_LOCK.read()){
                    motor_enable_pin.set_low();
                }
                if *executor::block_on(MOVE_LOCK.read()) && motor_enable_pin.is_set_low(){
                    motor_enable_pin.set_high();
                }
            }
            motor_enable_pin.set_low();
        }

        if limit_sense.is_low(){
            log::warn!("Fixture did not complete travel! Inspect fixture if this warning shows consistently.");
        }

        return limit_sense.is_high();
    }

    pub fn push_button(&mut self){
        if let Some(piston_enable) = self.piston_enable.as_mut(){
            while !*executor::block_on(MOVE_LOCK.read()){}
            piston_enable.set_high();
            thread::sleep(Duration::from_secs(1));
            piston_enable.set_low();
        }
    }
}
