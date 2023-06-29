use std::{collections::HashMap, 
          io::{BufReader, Write, Read}, 
          boxed::Box,
          time::Duration};
use serialport::SerialPort;

const BAUD_RATE:u32 = 115200;
const SERIAL_TIMEOUT: std::time::Duration = Duration::from_millis(500);
const FIRE_TEMP:&str="";

pub struct TTY{
    tty: Box<dyn SerialPort>,
}
impl std::fmt::Debug for TTY{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result{
        let absolute_location = self.tty.name();
        let relative_location:String;
        match absolute_location{
            Some(abs_location_string) => {
                let sectioned_abs_location = abs_location_string.rsplit_once('/');
                match sectioned_abs_location{
                    Some((_,serial_device_name)) => relative_location = serial_device_name.to_string(),
                    None => relative_location = "unknown".to_string()
                }
            },
            None => relative_location = "unknown".to_string()
        };
        f.debug_struct("TTY")
        .field("Serial port name",&relative_location)
        .finish()
    }
}

impl TTY{
    pub fn new(serial_location:&str) -> Option<Self>{
        let possible_tty = serialport::new(serial_location,BAUD_RATE).timeout(SERIAL_TIMEOUT).open();
        if let Ok(tty) = possible_tty{
            Some(TTY{tty})
        } else{
            None
        }
    }

    pub fn fire_temp(&mut self) -> bool {
        //if command == self.last{
        //    log::trace!("retry send {}",self.tty.name().unwrap_or("unknown".to_string()));
        //}else{
        //    log::debug!("writing {:?} to tty {}...", command, self.tty.name().unwrap_or("unknown".to_string()));
        //};
        let output = self.tty.write_all(FIRE_TEMP.as_bytes()).is_ok();
        _ = self.tty.flush();
        std::thread::sleep(SERIAL_TIMEOUT);
        return output;
    }

    pub fn read_from_device(&mut self) -> Option<f64> {
        let mut reader = BufReader::new(&mut self.tty);
        let mut read_buffer: Vec<u8> = Vec::new();
        _ = reader.read_to_end(&mut read_buffer);
        if read_buffer.len() > 0 {
            //NOTE: This may have to be removed
            let read_line:String = String::from_utf8_lossy(read_buffer.as_slice()).to_string();
            if read_line.eq("\r\n") {
                return None;
            } 
            if read_line.trim().eq(FIRE_TEMP){
                return self.read_from_device();
            }
            log::trace!("Successful read of {:?} from tty {}",read_line,self.tty.name().unwrap_or("unknown shell".to_string()));
            let mut lines = read_line.lines();
            while let Some(single_line) = lines.next(){
                let trimmed_line = single_line.trim();
                //Bitstream parsing
                todo!();
            }
            log::trace!("Unable to determine response. Response string is: [{:?}]",read_line);
            return None;
        }
        else {
            log::trace!("Read an empty string from device {:?}. Possible read error.", self);
            return None;
        };
    }
}
