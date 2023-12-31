use std::{collections::{HashMap,BTreeMap}, sync::Mutex};
use ini::Ini;
use chrono::Local;

//According to IEEE-754, floats can contain NaN, which is weird because:
//NaN != 0, (NaN < 0) == false, (NaN > 0) == false
//This means that Rust by default does not order floats.
//This crate fixes this shortcoming.
use ordered_float::OrderedFloat;

//Text file headers
const ITERATION_COUNT:&str="iterations completed this run";
const PASS_COUNT:&str="passing iterations";
const PASS_PERCENT:&str="Pass %";

const DEFAULT_LOWER:f32=35.8;
const DEFAULT_UPPER:f32=36.2;

pub struct OutputFile{
    file:Ini,
    filename:String,
}

pub struct TestState{
    //Mutex: Only modifiable by one thread at a time
    //HashMap: Key = String (device name), Value = TreeMap
    //  TreeMap: Key = float (measured value), Value = integer (how many times we've seen it)
    //                  TreeMaps require keys to be ordered, thus OrderedFloat
    data_map: Mutex<HashMap<String,BTreeMap<OrderedFloat<f32>,u64>>>
}

impl TestState{
    //TestState Constructor.
    pub fn new(device_names:Vec<String>) -> Self{
        let output = Self{
            data_map:Mutex::new(HashMap::new())
        };
        //initialise hashmap with the input device names
        device_names.iter()
            .for_each(|device| 
                _ = output.data_map.lock().unwrap().insert(
                    device.trim().trim_end_matches("\0").to_string(), BTreeMap::new()
                )
            );
        return output;
    }

    pub fn add_iteration(&self,device_name:String, value:f32){
        let name = device_name.trim().trim_end_matches("\0");
        let mut all_data = self.data_map.lock().unwrap();
        //if the device passed in doesn't exist yet in the HashMap, make it
        if !all_data.contains_key(&name.to_string()){
            all_data.insert(name.to_string(),BTreeMap::new());
        }
        //Device object should be created at this point, unwrap is safe
        let device_data = all_data.get_mut(&name.to_string()).unwrap();
        //If this value has never been found before, then create it as the first iteration
        if !device_data.contains_key(&value.into()){
            device_data.insert(OrderedFloat(value), 1);
        }
        //If the value has been seen before, just increment the counter
        else{
            //Value object should be created at this point, unwrap is safe
            let stored_value = device_data.get_mut(&value.into()).unwrap();
            *stored_value += 1;
        }
    }

    pub fn get_data(&self) -> HashMap<String,BTreeMap<OrderedFloat<f32>,u64>>{
        //Dump a copy of the hashmap
        self.data_map.lock().unwrap().clone()
    }
}

impl OutputFile{
    //OutputFile Constructor
    pub fn new(device_names:Vec<String>) -> Self{
        let mut config = Ini::new();
        //Init the Ini config with all devices
        for name in device_names{
            config.with_section(Some(name.trim().trim_end_matches("\0")))
                  .set(ITERATION_COUNT,0.to_string())
                  .set(PASS_COUNT,0.to_string());
        };
        let mut filename = String::from("output/");
        //Create a windows-safe filename with the format:
        //YYYY-MM-DD.HH_MM.txt
        filename.push_str(&Local::now().to_rfc3339());
        filename.truncate(23);
        filename = filename.replace(":","_");
        filename = filename.replace("T",".");
        filename.push_str(".txt");
        //Save a "blank" formatted file
        _ = config.write_to_file(&filename);
        //Return the created and initialised OutputFile
        Self{
            file:config,
            filename,
        }
    }
    
    pub fn write_values(&mut self, current_state:&TestState,upper_bound:Option<f32>,lower_bound:Option<f32>){
        let local_upper:f32;
        let local_lower:f32;
        //if the user didn't specify bounds, fallback to defaults
        match upper_bound{
            None => local_upper = DEFAULT_UPPER,
            Some(forced_upper) => local_upper = forced_upper,
        };
        match lower_bound{
            None => local_lower = DEFAULT_LOWER,
            Some(forced_lower) => local_lower = forced_lower,
        };

        //get the hashmap from the current state
        let data_map = current_state.get_data();
        
        //For each device...
        data_map.iter().for_each(|(device,value_map)|{
            //Get all values read from the device, and sort it
            let mut value_list:Vec<&OrderedFloat<f32>> = value_map.keys().collect();
            value_list.sort();

            let mut iteration_count = 0;
            let mut sum = 0;
            let mut pass_iteration_count = 0;
            let saved_data = &mut self.file;

            //For each value read from the device...
            value_map.iter().for_each(|(value,count)|{
                //Keep track of the total iterations, and the pass count
                iteration_count += count;
                if value.into_inner() > local_lower && value.into_inner() < local_upper{
                    pass_iteration_count += count;
                }

                //LEGACY CODE: sum is no longer being used
                sum += (value.into_inner() * *count as f32) as u128;

                //Add this value to the ini object
                saved_data.with_section(Some(&(device.to_owned() + " read value counts").to_string())).set(&value.to_string(),&count.to_string());
            });

            //Calculate pass percent
            let pass_percent= (iteration_count - (iteration_count - pass_iteration_count)) / iteration_count;

            //Add Pass percent, pass iteration count, and iteration count to ini object
            saved_data.with_section(Some(device))
                .set(PASS_PERCENT, pass_percent.to_string())
                .set(ITERATION_COUNT, iteration_count.to_string())
                .set(PASS_COUNT,pass_iteration_count.to_string());
        });

        //Flush ini object to text file
        _ = self.file.write_to_file(self.filename.clone());
    }
}
