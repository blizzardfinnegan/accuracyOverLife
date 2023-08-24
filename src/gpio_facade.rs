use std::time::Duration;
use async_std::sync::*;
use rppal::gpio::{Gpio,OutputPin,InputPin,Trigger,Level};
use std::thread;
use std::result::Result;
use futures::executor;
use std::fmt;

//10ms delay
const POLL_DELAY:Duration = Duration::from_millis(10);

//address of the motor enable pin
const MOTOR_ENABLE_ADDR:u8 = 22;

//address of the motor direction pin
const MOTOR_DIRECTION_ADDR:u8 = 27;

//Address of the air cylinder pin; used to press the button on the disco
const PISTON_ADDR:u8 = 25;

//address of the physical run switch on the fixture
//Used to interrupt fixture movement
const RUN_SWITCH_ADDR:u8 = 10;

//Raspberry Pi GPIO is a little finicky. 
//Active-high pins can occasionally drift high.
//Active-low portion of the switches are used as checks on the active-high pins.
//------
//address of Upper limit switch 
const UPPER_LIMIT_ADDR:u8 = 23;

//Address of upper limit switch 
const UPPER_NC_LIMIT_ADDR:u8 = 5;

//address of lower limit switch 
const LOWER_LIMIT_ADDR:u8 = 24;

//Address of lower limit switch 
const LOWER_NC_LIMIT_ADDR:u8 = 6;

//Boolean used to store whether the fixture is safe to move.
//This is stored in a RwLock to ensure that it is modifiable across threads
static MOVE_LOCK:RwLock<bool> = RwLock::new(true);

//Fixture struct definition
pub struct Fixture{
    gpio_api:Gpio,
    //Motor Direction: 
    //  Low = fixture travels down
    //  High = fixture travels up
    motor_direction:Option<OutputPin>,
    //Motor Enable: 
    //  Low = fixture doesn't travel
    //  High = fixture travels
    motor_enable: Option<OutputPin>,
    //Piston enable:
    //  Low = Piston retracted; button is not pressed
    //  high = piston extended; button is pressed
    piston_enable: Option<OutputPin>,
    //Upper Limit switch [active high]:
    //  Low = Upper limit switch is triggered
    //  high = upper limit switch has not been triggered yet
    upper_limit: Option<InputPin>,
    //Upper Limit switch [active low]:
    //  Low = upper limit switch has not been triggered yet
    //  high = Upper limit switch is triggered
    upper_nc_limit: Option<InputPin>,
    //Lower Limit switch [active high]:
    //  Low = Lower limit switch is triggered
    //  high = Lower limit switch has not been triggered yet
    lower_limit: Option<InputPin>,
    //Lower Limit switch [active low]:
    //  Low = Lower limit switch has not been triggered yet
    //  high = Lower limit switch is triggered
    lower_nc_limit: Option<InputPin>,
}

//Possible fixture movement directions
pub enum Direction{Up,Down}

//Reset arm on close
impl Drop for Fixture{
    fn drop(&mut self) {
        self.reset_arm();
    }
}

//Custom error for initialisation
#[derive(Debug,Clone)]
pub struct FixtureInitError;

//to_string for the above error
impl fmt::Display for FixtureInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid fixture PWM value")
    }
}

//Future Improvement: modify implementation to allow for multiple fixtures simultaneously
impl Fixture{
    //Fixture Constructor
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

            //Output [control] pin ceation; initialise all as low [off]
            //-------------
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

            //Input [sense] pin creation; initialise all as active-high
            //-------------
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

            //Initialise run switch as sense pin as active-high
            match output.gpio_api.get(RUN_SWITCH_ADDR){
                Ok(run_pin) =>{
                    //Use ethe switch as an asynchronous interrupt
                    _ = run_pin.into_input_pulldown().set_async_interrupt(Trigger::Both, |switch_state|{
                        //Block fixture movement when the run switch is low
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

            //If GPIO is completely initialised properly, reset the arm to the top of 
            //the fixture, then test the fixture's range of motion, before returning the fixture
            //object
            output.reset_arm();
            output.goto_limit(Direction::Down);
            output.goto_limit(Direction::Up);
            return Ok(output);

        } // end if Ok(gpio) = possible_gpio
        else { 
            //Most errors can be resolved by running the software as root
            log::error!("Gpio could not be opened! Did you run with 'sudo'? ");
            return Err(FixtureInitError) 
        }
    }

    //Function to reset the arm
    //returns how many polls it took to reset [polled at 10ms intervals]
    //returns 0 if fixture could not be triggered
    //Note: Polling can probably be removed at this time
    fn reset_arm(&mut self) -> u16{
        log::debug!("Resetting arm...");
        if let (Some(upper_limit),Some(upper_nc_limit),          Some(motor_direction_pin),      Some(motor_enable_pin)) = 
               (self.upper_limit.as_mut(),self.upper_nc_limit.as_mut(), self.motor_direction.as_mut(), self.motor_enable.as_mut()){
            log::trace!("Upper limit: {}",upper_limit.is_high());
            log::trace!("Upper NC limit: {}",upper_nc_limit.is_high());
            //If the fixture believes it is at the top of the fixture, send the fixture down
            //briefly to re-confirm it is at the top
            if upper_limit.is_high(){
                {
                    //Wait until run switch says its safe to go
                    while !*executor::block_on(MOVE_LOCK.read()) {log::trace!("blocking!");}
                    motor_direction_pin.set_low();
                    motor_enable_pin.set_high()
                }
                //Stop the motor once its traveled for 0.5s
                thread::sleep(Duration::from_millis(500));
                motor_enable_pin.set_low()
            }
            //Once its safe, start travelling up
            while !*executor::block_on(MOVE_LOCK.read()){log::trace!("blocking!");}
            motor_direction_pin.set_high();
            motor_enable_pin.set_high();
            let mut counter = 0;
            //Every 10ms, check if the fixture is done travelling
            while upper_limit.is_low() && upper_nc_limit.is_high() {
                //If the run switch is flipped during movement, immediately pause
                while !*executor::block_on(MOVE_LOCK.read()){
                    motor_enable_pin.set_low();
                }
                //Recover once the run switch is reset
                if *executor::block_on(MOVE_LOCK.read()) && motor_enable_pin.is_set_low(){
                    motor_enable_pin.set_high();
                }
                //This probably shouldn't be logging
                if upper_limit.is_high() && upper_nc_limit.is_low() { 
                    log::trace!("Breaking early!");
                    break; 
                }
                counter += 1;
                thread::sleep(POLL_DELAY);
            }
            //Stop moving once the fixture is back at the highest point; return the number of
            //polls
            motor_enable_pin.set_low();
            return counter;
        };
        return 0;
    }

    //Go to either the top or bottom of the fixure's mmovement
    //Returns `bool` of whether the movement was successful
    pub fn goto_limit(&mut self, direction:Direction) -> bool{
        let ref mut limit_sense:InputPin;
        let ref mut limit_nc_sense:InputPin;
        //Movement is the same idea in either direction; only difference is which limit senses
        //we're listening to
        match direction{
            Direction::Down => {
                log::trace!("Sending fixture down...");
                if let (Some(obj),Some(obj2),Some(motor_direction_pin)) = (self.lower_limit.as_mut(),self.lower_nc_limit.as_mut(),self.motor_direction.as_mut()) {
                    limit_sense = obj;
                    limit_nc_sense = obj2;
                    motor_direction_pin.set_low();
                }
                //If the fixture's GPIO pins haven't been initialised yet, or they can't be
                //accessed, obviously the fixture won't go anywhere; early return
                else { return false; }
            },
            Direction::Up => {
                log::trace!("Sending fixture up...");
                if let (Some(obj),Some(obj2),Some(motor_direction_pin)) = (self.upper_limit.as_mut(),self.upper_nc_limit.as_mut(),self.motor_direction.as_mut()) {
                    limit_sense = obj;
                    limit_nc_sense = obj2;
                    motor_direction_pin.set_high();
                }
                //If the fixture's GPIO pins haven't been initialised yet, or they can't be
                //accessed, obviously the fixture won't go anywhere; early return
                else { return false; }
            }
        }

        //If we're already at the limit switch, no reason to break the fixture. Technically, a
        //successful fixture movement, return true
        if limit_sense.is_high() && limit_nc_sense.is_low(){ log::debug!("Fixture already at proper limit switch!"); return true; }

        //Move the fixture until its at the proper limit switch
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

        //LEGACY CHECK: This is covered by the active-low pin, can be safely removed
        if limit_sense.is_low(){
            log::warn!("Fixture did not complete travel! Inspect fixture if this warning shows consistently.");
        }

        //Should probably be changed to be the same as the premature successful movement's return
        //status
        return limit_sense.is_high();
    }

    //Extend the piston for 0.25s
    pub fn push_button(&mut self){
        if let Some(piston_enable) = self.piston_enable.as_mut(){
            while !*executor::block_on(MOVE_LOCK.read()){}
            piston_enable.set_high();
            thread::sleep(Duration::from_millis(250));
            piston_enable.set_low();
        }
    }
}
