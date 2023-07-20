use std::{io::{BufReader, Write, Read}, 
          boxed::Box,
          time::Duration};
use serialport::SerialPort;

const BAUD_RATE:u32 = 115200;
const SERIAL_TIMEOUT: std::time::Duration = Duration::from_millis(50);


///----------------------
/// For more information on the below constants, see WACP Spec documentation.
///----------------------

//Request currently shown temp from device 
const REQUEST_TEMP:   &[u8; 26]= b"\x17\x01\x0c\x00\x00\x00\x1a\x01\x19\x00\x03\x0b\x00\x00\x00\x00\x07\x00\x00\x00\x00\x00\xe9\x32\x94\xfe";

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
            if tty.write_all(REQUEST_SERIAL).is_ok(){
                _ = tty.flush();
                let mut reader = BufReader::new(&mut tty);
                let mut read_buffer: Vec<u8> = Vec::new();
                _ = reader.read_to_end(&mut read_buffer);
                let serial:String = TTY::parse_serial_response(read_buffer);
                Some(TTY{tty,serial})
            }
            else{
                None
            }
        } else{
            None
        }
    }

    fn parse_serial_response(read_buffer:Vec<u8>) -> String{
        log::trace!("Requesting serial...");
        //Serial response packet is 147 bytes long
        if read_buffer.len() == 147{
            let buffer = read_buffer.clone();
            let mut serial:String = String::new();

            let mut buffer_index:usize = 0;
            //The preamble is weird, and is only 3 bytes long. Putting it in a u32, and setting the
            //first octet to 0
            let preamble = u32::from_be_bytes([0x00,buffer[buffer_index],buffer[buffer_index + 1],buffer[buffer_index + 2]]);
            buffer_index += 3;
            //Predefined WACP Preamble
            if preamble != 0x17010c {
                log::error!("No preamble found! Bad packet. Returning null.");
            }

            let expected_packet_size = TTY::u32_from_bytes(&buffer, &mut buffer_index);
            if expected_packet_size as usize != buffer.len(){
                log::error!("Bad packet size!!! Expected size: {}, Actual size: {}",
                                            expected_packet_size,buffer.len());
            }

            //Buffer[7..8] are port numbers and not actually important for anything for our purposes
            buffer_index += 2;
            
            let msg_class_id = TTY::u32_from_bytes(&buffer, &mut buffer_index);
            //Expected message class: Temperature Response [assumed]
            if msg_class_id != 0x00180f00 {
                log::error!("Unknown message response class: {}. Expected: 1576704. See WACP documentation.",msg_class_id);
            }

            let msg_size = TTY::u32_from_bytes(&buffer, &mut buffer_index);
            //Bytes counted in packet length but not in msg length: 19
            if msg_size as usize != (buffer.len() - 19) {
                log::error!("Bad message size! Expected size: {}, Actual size: {}",
                                                msg_size, buffer.len() - 19);
            }

            let encrypted = TTY::u8_from_bytes(&buffer, &mut buffer_index);
            //Encryption bytes are not implemented as of now, and this code does not know how to
            //interpret encrypted or compressed data.
            if encrypted != 0x0{
                log::error!("Message potentially encrypted! Consult documentation concerning bitmask {}!",encrypted);
            }

            //Bytes counted in packet length but not in msg length: 19
            //Bytes counted in msg length but not in obj length:     7
            let obj_size = TTY::u32_from_bytes(&buffer, &mut buffer_index);
            if obj_size as usize != (buffer.len() - 26) {
                log::error!("Bad object size! Expected size: {}, Actual size: {}",
                                                obj_size, buffer.len() - 26);
            }

            let obj_id = TTY::u32_from_bytes(&buffer, &mut buffer_index);
            //ObjectID = CTempDData
            if obj_id != 0x00180000{
                log::error!("Unknown object ID: {} Consult documentation.",obj_id);
            }

            let obj_size_no_header = TTY::u16_from_bytes(&buffer, &mut buffer_index);
            //Bytes counted in packet length but not in msg length: 19
            //Bytes counted in msg length but not in obj length:     7
            //bytes counted in obj length but not obj. internal len: 6
            if obj_size_no_header as usize != (buffer.len() - 32){
                log::error!("Bad object inner size! Expected size: {}, Actual size: {}",
                                        obj_size_no_header, buffer.len() - 32);
            };

            let obj_version = TTY::u16_from_bytes(&buffer, &mut buffer_index);
            //This code is written to interpret version 205. Versions are backwards compatible, at
            //time of writing.
            if obj_version > 0x00cd{
                log::error!("Object version newer than expected! ({} > 205) Manually check response.",obj_version);
                panic!("Unexpected object version. Here be dragons.");
            };

            let obj_bitmask = TTY::u8_from_bytes(&buffer, &mut buffer_index);
            //Encryption bytes are not implemented as of now, and this code does not know how to
            //interpret encrypted or compressed data.
            if obj_bitmask != 0x00{
                log::error!("Bad object bitmask! Consult documentation concerning bitmask {}.",obj_bitmask);
            };

            let static_size = TTY::u16_from_bytes(&buffer, &mut buffer_index);
            //Static data in this packet type is expected to take 16 bytes.
            if static_size != 0x006c{
                log::error!("Unexpected static variable size ({} != 108). Manually check response.",static_size);
                panic!("Unexpected static variable size. Here be dragons.");
            };

            for char_val in buffer[77..93].into_iter(){
                serial.push(char::from(char_val.clone()));
            };
            return serial.trim().trim_end_matches("\0").to_string();
        };
        return "Invalid device!".to_string();
    }

    pub fn get_serial(&mut self) -> &str { &self.serial }

    pub fn get_temp(&mut self) -> Option<f32> {
        let output = self.tty.write_all(REQUEST_TEMP).is_ok();
        _ = self.tty.flush();
        std::thread::sleep(SERIAL_TIMEOUT);
        if output{
            let mut reader = BufReader::new(&mut self.tty);
            let mut read_buffer: Vec<u8> = Vec::new();
            _ = reader.read_to_end(&mut read_buffer);
            if read_buffer.len() == 78 {
                let buffer = read_buffer.clone();
                let mut buffer_index = 0;

                //from_be_bytes : From Big-Endian bytes.
                let preamble = u32::from_be_bytes([0x00,buffer[0],buffer[1],buffer[2]]);
                buffer_index += 3;
                //Predefined WACP Preamble
                if preamble != 0x17010c {
                    log::error!("No preamble found! Bad packet.");
                    return self.get_temp();
                }
                let expected_packet_size = TTY::u32_from_bytes(&buffer, &mut buffer_index);
                if expected_packet_size as usize != buffer.len(){
                    log::error!("Bad packet size!!! Expected size: {}, Actual size: {}",
                                                expected_packet_size,buffer.len());
                    panic!("Bad packet size!");
                }

                //Buffer[7..8] are port numbers and not actually important for anything for our purposes
                buffer_index += 2;
                
                let msg_class_id = TTY::u32_from_bytes(&buffer, &mut buffer_index);
                //Expected message class: Temperature Response [assumed]
                if msg_class_id != 0x00030f00 {
                    log::error!("Unknown message response class: {}. See WACP documentation.",msg_class_id);
                    panic!("Unknown message response class!");
                }

                let msg_size = TTY::u32_from_bytes(&buffer, &mut buffer_index);
                //Bytes counted in packet length but not in msg length: 19
                if msg_size as usize != (buffer.len() - 19) {
                    log::error!("Bad message size! Expected size: {}, Actual size: {}",
                                                    msg_size, buffer.len() - 19);
                    panic!("Bad message size!");
                }

                let encrypted = TTY::u8_from_bytes(&buffer, &mut buffer_index);
                //Encryption bytes are not implemented as of now, and this code does not know how to
                //interpret encrypted or compressed data.
                if encrypted != 0x0{
                    log::error!("Message potentially encrypted! Consult documentation concerning bitmask {}!",encrypted);
                    panic!("Potentially encrypted information, unable to continue.");
                }

                let obj_size = TTY::u32_from_bytes(&buffer, &mut buffer_index);
                //Bytes counted in packet length but not in msg length: 19
                //Bytes counted in msg length but not in obj length:     7
                if obj_size as usize != (buffer.len() - 26) {
                    log::error!("Bad object size! Expected size: {}, Actual size: {}",
                                                    obj_size, buffer.len() - 26);
                    panic!("Bad object size!");
                }

                let obj_id = TTY::u32_from_bytes(&buffer, &mut buffer_index);
                //ObjectID = CTempDData
                if obj_id != 0x00030001{
                    log::error!("Unknown object ID: {} Consult documentation.",obj_id);
                    panic!("Unknown object ID!");
                }

                let obj_size_no_header = TTY::u16_from_bytes(&buffer, &mut buffer_index);
                //Bytes counted in packet length but not in msg length: 19
                //Bytes counted in msg length but not in obj length:     7
                //bytes counted in obj length but not obj. internal len: 6
                if obj_size_no_header as usize != (buffer.len() - 32){
                    log::error!("Bad object internal size! Expected size: {}, Actual size: {}",
                                            obj_size_no_header, buffer.len() - 32);
                    panic!("Bad object internal size!");
                };

                let obj_version = TTY::u16_from_bytes(&buffer, &mut buffer_index);
                //This code is written to interpret version 205. Versions are backwards compatible, at
                //time of writing.
                if obj_version > 0x00cd{
                    log::error!("Object version newer than expected! Manually check response.");
                    panic!("Unexpected object version. Here be dragons.");
                };

                let obj_bitmask = TTY::u8_from_bytes(&buffer, &mut buffer_index);
                //Encryption bytes are not implemented as of now, and this code does not know how to
                //interpret encrypted or compressed data.
                if obj_bitmask != 0x00{
                    log::error!("Bad inner object bitmask! Consult documentation concerning bitmask {}.",obj_bitmask);
                    panic!("Unexpected inner object bitmask!");
                };

                let static_size = TTY::u16_from_bytes(&buffer, &mut buffer_index);
                //Static data in this packet type is expected to take 16 bytes.
                if static_size != 0x0010{
                    log::error!("Unexpected static variable size. Manually check response.");
                    panic!("Unexpected static variable size. Here be dragons.");
                };

                let time = TTY::u64_from_bytes(&buffer, &mut buffer_index);
                if time != 0x0{
                    log::error!("Unexpected time value recieved from disco! Manually check response!");
                    panic!("Unexpected time value!");
                }

                let status = TTY::u16_from_bytes(&buffer, &mut buffer_index);
                if status == 0x00{
                    //log::error!("Data is not available!");
                }
                else if status != 0x01{
                    log::error!("Unexpected status value!");
                }

                let extended_status = TTY::u16_from_bytes(&buffer, &mut buffer_index);
                if extended_status != 0x0{
                    log::error!("Unexpected extended status! {}",extended_status);
                }
                
                let source   = TTY::u16_from_bytes(&buffer, &mut buffer_index);
                if source != 0x0f{
                    log::error!("unexpected device response! Expected source is Disco (0x0f), device reports as {}",source);
                    panic!("Unexpected source!");
                };

                let op_mode = TTY::u8_from_bytes(&buffer, &mut buffer_index);
                if op_mode != 0x0f{
                    log::error!("Unexpected operation mode. Temperature is not trustworthy. Expected op mode is tympanic (0x0f), device reports as {}",op_mode);
                    panic!("Unexpected operation mode!");
                };

                let calc_method = TTY::u8_from_bytes(&buffer, &mut buffer_index);
                if calc_method == 0x0d {
                    log::error!("Device has fallen out of unadjusted mode!");
                } 
                else if calc_method != 0x0c {
                    log::error!("Device is generating data with unknown calculation method!");
                };
                
                let encapsulated_obj_size = TTY::u32_from_bytes(&buffer, &mut buffer_index);
                //Bytes counted in packet length but not in msg length: 19
                //Bytes counted in msg length but not in obj length:     7
                //bytes counted in obj len but not obj. internal len:    6
                //bytes counted in object but not in encapsulated obj.: 27
                if encapsulated_obj_size as usize != buffer.len() - 59{
                    log::error!("Unexpected encapsulated object size! Expected size: {}, actual size: {}", 
                                                                encapsulated_obj_size, buffer.len() - 59);
                    panic!("Unexpected encapsulated object size!");
                };

                let encap_obj_id = TTY::u32_from_bytes(&buffer, &mut buffer_index);
                if encap_obj_id != 0x0075001f {
                    log::error!("Unexpected encapsulated object ID! Please manually check response.");
                    panic!("Unexpected encapsulated object ID. Here be dragons.");
                };

                //Bytes counted in packet length but not in msg length:     19
                //Bytes counted in msg length but not in obj length:         7
                //bytes counted in obj len but not obj. internal len:        6
                //bytes counted in object but not in encapsulated obj.:     27
                //bytes counted in encap obj but not in encap obj int. len: 13
                let encap_obj_size = TTY::u16_from_bytes(&buffer, &mut buffer_index);
                if encap_obj_size as usize != buffer.len() - 72{
                    //log::error!("Bad encapsulated object size! Expected size: {}, actual size: {}",encap_obj_size, buffer.len() - 72);
                    //panic!("Unexpected encapsulated object size!");
                };

                let encap_obj_version = TTY::u16_from_bytes(&buffer, &mut buffer_index);
                if encap_obj_version > 0x00c8{
                    log::error!("Encapsulated object newer version than expected. Manually check response.");
                    panic!("Response contains too new of an encapsulated object version.");
                };

                let encap_obj_bitmask = TTY::u8_from_bytes(&buffer, &mut buffer_index);
                if encap_obj_bitmask != 0x00{
                    log::error!("Encapsulated object contains unknown bitmask. Check documentation for bitmask {}",encap_obj_bitmask);
                    panic!("Bad encapsulated object bitmask.");
                };

                let encap_obj_var_size = TTY::u16_from_bytes(&buffer, &mut buffer_index);
                //encapsulated object is CNumDFloat. That is, a float [4 byte], followed by a 2 byte status bitmask
                if encap_obj_var_size != 6{
                    log::error!("Encapsulated object is wrong size for CNumDFloat! Manually check response.");
                    panic!("Encapsulated object's static variable size is an unexpected value. [Not 6]");
                };

                let temp = TTY::f32_from_bytes(&buffer, &mut buffer_index);

                let disco_temp_status = TTY::u16_from_bytes(&buffer, &mut buffer_index);
                if disco_temp_status ^ 0x01 != 0{
                    if disco_temp_status == 0x80{
                        log::trace!("Calculations incomplete, waiting for response...");
                        return self.get_temp();
                    }
                    log::error!("Disco temp status unimplemented! Returned disco status: {}",disco_temp_status);
                    todo!();
                };

                //The value the Disco reports is in Kelvin. Convert to Celsius for easier comparison
                //with bounds.
                return Some(temp - 273.15);
            }
            else {
                log::trace!("Read an empty string from device {:?}. Possible read error.", self);
                return None;
            };
        };
        return None;
    }

    fn f32_from_bytes(bytes:&Vec<u8>, index:&mut usize) -> f32{
        //from_be_bytes : From Big-Endian bytes.
        let output:f32 = 
            f32::from_be_bytes([*bytes.get(*index    ).unwrap_or(&0),
                                *bytes.get(*index + 1).unwrap_or(&0),
                                *bytes.get(*index + 2).unwrap_or(&0),
                                *bytes.get(*index + 3).unwrap_or(&0)]);
        //Increment index to next value
        *index += 4;
        return output
    }

    fn u64_from_bytes(bytes:&Vec<u8>, index:&mut usize) -> u64{
        //from_be_bytes : From Big-Endian bytes.
        let output:u64 = 
            u64::from_be_bytes([*bytes.get(*index    ).unwrap_or(&0),
                                *bytes.get(*index + 1).unwrap_or(&0),
                                *bytes.get(*index + 2).unwrap_or(&0),
                                *bytes.get(*index + 3).unwrap_or(&0),
                                *bytes.get(*index + 4).unwrap_or(&0),
                                *bytes.get(*index + 5).unwrap_or(&0),
                                *bytes.get(*index + 6).unwrap_or(&0),
                                *bytes.get(*index + 7).unwrap_or(&0)]);
        //Increment index to next value
        *index += 8;
        return output
    }

    fn u32_from_bytes(bytes:&Vec<u8>, index:&mut usize) -> u32{
        //from_be_bytes : From Big-Endian bytes.
        let output:u32 = 
            u32::from_be_bytes([*bytes.get(*index    ).unwrap_or(&0),
                                *bytes.get(*index + 1).unwrap_or(&0),
                                *bytes.get(*index + 2).unwrap_or(&0),
                                *bytes.get(*index + 3).unwrap_or(&0)]);
        //Increment index to next value
        *index += 4;
        return output
    }

    fn u16_from_bytes(bytes:&Vec<u8>, index:&mut usize) -> u16{
        //from_be_bytes : From Big-Endian bytes.
        let output:u16 = 
            u16::from_be_bytes([*bytes.get(*index    ).unwrap_or(&0),
                                *bytes.get(*index + 1).unwrap_or(&0)]);
        *index += 2;
        return output
    }

    fn u8_from_bytes(bytes:&Vec<u8>,index:&mut usize) -> u8{
        //Increment index to next value
        *index += 1;
        return *bytes.get(*index - 1).unwrap_or(&0);
    }
}
