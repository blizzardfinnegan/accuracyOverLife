use std::{io::{BufReader, Write, Read}, 
          boxed::Box,
          time::Duration};
use serialport::SerialPort;

const BAUD_RATE:u32 = 115200;
const SERIAL_TIMEOUT: std::time::Duration = Duration::from_millis(500);


///----------------------
/// For more information on the below constants, see WACP Spec documentation.
///----------------------

//Request currently shown temp from device 
const REQUEST_TEMP: &[u8; 26]= b"\x17\x01\x0c\x00\x00\x00\x1a\x01\x19\x00\x03\x0b\x00\x00\x00\x00\x07\x00\x00\x00\x00\x00\xe9\x32\x94\xfe";

//Request a device's serial
const REQUEST_SERIAL: &[u8; 26]= b"\x17\x01\x0c\x00\x00\x00\x1a\x01\x19\x00\x18\x0b\x00\x00\x00\x00\x07\x00\x00\x00\x00\x00\x71\xe8\x80\x3e";

pub struct TTY{
    tty: Box<dyn SerialPort>,
    serial: String
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
        .field("Serial port location",&relative_location)
        .field("device name",&self.serial)
        .finish()
    }
}

impl TTY{
    pub fn new(serial_location:&str) -> Option<Self>{
        let possible_tty = serialport::new(serial_location,BAUD_RATE).timeout(SERIAL_TIMEOUT).open();
        if let Ok(mut tty) = possible_tty{
            let mut serial:String = String::new();
            if tty.write_all(REQUEST_SERIAL).is_ok(){
                let mut reader = BufReader::new(&mut tty);
                let mut read_buffer: Vec<u8> = Vec::new();
                _ = reader.read_to_end(&mut read_buffer);
                if read_buffer.len() == 147{
                    let buffer = read_buffer.as_slice();
                    for char_val in buffer[45..77].into_iter(){
                        serial.push(char::from(char_val.clone()));
                    };
                    Some(TTY{tty,serial})
                }
                else{
                    return None
                }
            }
            else{
                None
            }
        } else{
            None
        }
    }

    pub fn get_serial(&mut self) -> String { self.serial.clone() }

    pub fn get_temp(&mut self) -> Option<f32> {
        let output = self.tty.write_all(REQUEST_TEMP).is_ok();
        _ = self.tty.flush();
        std::thread::sleep(SERIAL_TIMEOUT);
        if output{
            let mut reader = BufReader::new(&mut self.tty);
            let mut read_buffer: Vec<u8> = Vec::new();
            _ = reader.read_to_end(&mut read_buffer);
            if read_buffer.len() == 78 {
                let buffer = read_buffer.as_slice();

                //from_be_bytes : From Big-Endian bytes.
                let preamble = u32::from_be_bytes([0x00,buffer[0],buffer[1],buffer[2]]);
                //Predefined WACP Preamble
                if preamble != 0x17010c {
                    log::error!("No preamble found! Bad packet. Returning null.");
                    return None;
                }
                let expected_packet_size = 
                        u32::from_be_bytes([buffer[3],buffer[4],buffer[5],buffer[6]]);
                if expected_packet_size as usize != buffer.len(){
                    log::error!("Bad packet size!!! Expected size: {}, Actual size: {}",
                                                expected_packet_size,buffer.len());
                    return None;
                }

                //Buffer[7..8] are port numbers and not actually important for anything for our purposes
                
                let msg_class_id = u32::from_be_bytes([buffer[9],buffer[10],buffer[11],buffer[12]]);
                //Expected message class: Temperature Response [assumed]
                if msg_class_id != 0x00030f00 {
                    log::error!("Unknown message response class: {}. See WACP documentation.",msg_class_id);
                    return None;
                }

                let msg_size = u32::from_be_bytes([buffer[13],buffer[14],buffer[15],buffer[16]]);
                //Bytes counted in packet length but not in msg length: 19
                if msg_size as usize != (buffer.len() - 19) {
                    log::error!("Bad message size! Expected size: {}, Actual size: {}",
                                                    msg_size, buffer.len() - 19);
                    return None;
                }

                let encrypted = buffer[17];//u16::from_be_bytes([buffer[17],buffer[18]]);
                //Encryption bytes are not implemented as of now, and this code does not know how to
                //interpret encrypted or compressed data.
                if encrypted != 0x0{
                    log::error!("Message potentially encrypted! Consult documentation concerning bitmask {}!",encrypted);
                }

                //Bytes counted in packet length but not in msg length: 19
                //Bytes counted in msg length but not in obj length:     7
                let obj_size = u32::from_be_bytes([buffer[18],buffer[19],buffer[20],buffer[21]]);
                if obj_size as usize != (buffer.len() - 26) {
                    log::error!("Bad object size! Expected size: {}, Actual size: {}",
                                                    obj_size, buffer.len() - 26);
                    return None;
                }

                let obj_id = u32::from_be_bytes([buffer[22],buffer[23],buffer[24],buffer[25]]);
                //ObjectID = CTempDData
                if obj_id != 0x00030001{
                    log::error!("Unknown object ID: {} Consult documentation.",obj_id);
                    return None;
                }

                let obj_size_no_header =
                        u16::from_be_bytes([buffer[26],buffer[27]]);
                //Bytes counted in packet length but not in msg length: 19
                //Bytes counted in msg length but not in obj length:     7
                //bytes counted in obj length but not obj. internal len: 6
                if obj_size_no_header as usize != (buffer.len() - 32){
                    log::error!("Bad object inner size! Expected size: {}, Actual size: {}",
                                            obj_size_no_header, buffer.len() - 32);
                    return None;
                };

                let obj_version = u16::from_be_bytes([buffer[28],buffer[29]]);
                //This code is written to interpret version 205. Versions are backwards compatible, at
                //time of writing.
                if obj_version > 0x00cd{
                    log::error!("Object version newer than expected! Manually check response.");
                    panic!("Unexpected object version. Here be dragons.");
                };

                let obj_bitmask = buffer[30];
                //Encryption bytes are not implemented as of now, and this code does not know how to
                //interpret encrypted or compressed data.
                if obj_bitmask != 0x00{
                    log::error!("Bad object bitmask! Consult documentation concerning bitmask {}.",obj_bitmask);
                    return None;
                };

                let static_size = u16::from_be_bytes([buffer[31],buffer[32]]);
                //Static data in this packet type is expected to take 16 bytes.
                if static_size != 0x0010{
                    log::error!("Unexpected static variable size. Manually check response.");
                    panic!("Unexpected static variable size. Here be dragons.");
                };

                //static values for time, status, and extended status [36-49] are currently unknown. Will add
                //values when known.
                
                let source      = buffer[46];
                if source != 0x0f{
                    log::error!("unexpected device response! Expected source is Disco (0x0f), device reports as {}",source);
                    return None;
                };

                let op_mode     = buffer[47];
                if op_mode != 0x0f{
                    log::error!("Unexpected operation mode. Temperature is not trustworthy. Expected op mode is tympanic (0x0f), device reports as {}",op_mode);
                    return None;
                };

                //let calc_method = buffer[48];
                //Unknown calc method response for unadjusted mode. will edit when known.
                
                let encapsulated_obj_size = u32::from_be_bytes([buffer[49],buffer[50],buffer[51],buffer[52]]);
                //Bytes counted in packet length but not in msg length: 19
                //Bytes counted in msg length but not in obj length:     7
                //bytes counted in obj len but not obj. internal len:    6
                //bytes counted in object but not in encapsulated obj.: 27
                if encapsulated_obj_size as usize != buffer.len() - 59{
                    log::error!("Unexpected encapsulated object size! Expected size: {}, actual size: {}", 
                                                                encapsulated_obj_size, buffer.len() - 59);
                    return None;
                };

                let encap_obj_id = u32::from_be_bytes([buffer[53],buffer[54],buffer[55],buffer[56]]);
                if encap_obj_id != 0x0075001f {
                    log::error!("Unexpected encapsulated object ID! Please manually check response.");
                    panic!("Unexpected encapsulated object ID. Here be dragons.");
                };

                //Bytes counted in packet length but not in msg length:     19
                //Bytes counted in msg length but not in obj length:         7
                //bytes counted in obj len but not obj. internal len:        6
                //bytes counted in object but not in encapsulated obj.:     27
                //bytes counted in encap obj but not in encap obj int. len: 13
                let encap_obj_size = u16::from_be_bytes([buffer[57],buffer[58]]);
                if encap_obj_size as usize != buffer.len() - 72{
                    log::error!("Bad encapsulated object size! Expected size: {}, actual size: {}",
                                                                encap_obj_size, buffer.len() - 72);
                    return None;
                };

                let encap_obj_version = u16::from_be_bytes([buffer[59],buffer[60]]);
                if encap_obj_version > 0x00c8{
                    log::error!("Encapsulated object newer version than expected. Manually check response.");
                    panic!("Response contains too new of an encapsulated object version.");
                };

                let encap_obj_bitmask = buffer[61];
                if encap_obj_bitmask != 0x00{
                    log::error!("Encapsulated object contains unknown bitmask. Check documentation for bitmask {}",encap_obj_bitmask);
                    panic!("Bad encapsulated object bitmask.");
                };

                let encap_obj_var_size = u16::from_be_bytes([buffer[62],buffer[63]]);
                //encapsulated object is CNumDFloat. That is, a float [4 byte], followed by a 2 byte status bitmask
                if encap_obj_var_size != 6{
                    log::error!("Encapsulated object is wrong size for CNumDFloat! Manually check response.");
                    panic!("Encapsulated object's static variable size is an unexpected value. [Not 6]");
                };

                let disco_temp_status = u16::from_be_bytes([buffer[68],buffer[69]]);
                if disco_temp_status ^ 0x0001 != 0{
                    log::error!("Unexpected disco status! Disco is now in status {}!",disco_temp_status);
                    todo!();
                };

                let temp = f32::from_be_bytes([buffer[64],buffer[65],buffer[66],buffer[67]]);
                //The value the Disco reports is in Kelvin. Convert to Celsius for easier comparison
                //with bounds.
                return Some(temp - 273.15);
            }
            else {
                log::trace!("Read an empty string from device {:?}. Possible read error.", self);
                return None;
            };
        };
        return None
    }
}
