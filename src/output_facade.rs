use std::{collections::{HashMap,BTreeMap}, sync::Mutex};
use ini::Ini;
use chrono::Local;
use ordered_float::OrderedFloat;

const ITERATION_COUNT:&str="iterations completed this run";
const PASS_COUNT:&str="passing iterations";
const DEFAULT_LOWER:f32=35.8;
const DEFAULT_UPPER:f32=36.2;
pub struct OutputFile{
    file:Ini,
    filename:String,
}

pub struct TestState{
    data_map: Mutex<HashMap<String,BTreeMap<OrderedFloat<f32>,u64>>>
}

impl TestState{
    pub fn new(device_names:Vec<String>) -> Self{
        let output = Self{
            data_map:Mutex::new(HashMap::new())
        };
        device_names.iter()
            .for_each(|device| _ = output.data_map.lock().unwrap().insert(device.trim().trim_end_matches("\0").to_string(),BTreeMap::new()));
        return output;
    }

    pub fn add_iteration(&self,device_name:String, value:f32){
        let name = device_name.trim().trim_end_matches("\0");
        let mut all_data = self.data_map.lock().unwrap();
        if !all_data.contains_key(&name.to_string()){
            all_data.insert(name.to_string(),BTreeMap::new());
        }
        //Device object should be created at this point, unwrap is safe
        let device_data = all_data.get_mut(&name.to_string()).unwrap();
        if !device_data.contains_key(&value.into()){
            device_data.insert(OrderedFloat(value), 1);
        }
        //Value object should be created at this point, unwrap is safe
        else{
            let stored_value = device_data.get_mut(&value.into()).unwrap();
            *stored_value += 1;
        }
    }

    pub fn get_data(&self) -> HashMap<String,BTreeMap<OrderedFloat<f32>,u64>>{
        self.data_map.lock().unwrap().clone()
    }
}

impl OutputFile{
    pub fn new(device_names:Vec<String>) -> Self{
        let mut config = Ini::new();
        for name in device_names{
            config.with_section(Some(name.trim().trim_end_matches("\0")))
                  .set(ITERATION_COUNT,0.to_string())
                  .set(PASS_COUNT,0.to_string());
        };
        let mut filename = String::from("output/");
        filename.push_str(&Local::now().to_rfc3339());
        filename.truncate(23);
        filename = filename.replace(":","_");
        filename = filename.replace("T",".");
        filename.push_str(".txt");
        _ = config.write_to_file(&filename);
        Self{
            file:config,
            filename,
        }
    }
    
    pub fn write_values(&mut self, current_state:&TestState,upper_bound:Option<f32>,lower_bound:Option<f32>){
        let local_upper:f32;
        let local_lower:f32;
        match upper_bound{
            None => local_upper = DEFAULT_UPPER,
            Some(forced_upper) => local_upper = forced_upper,
        };
        match lower_bound{
            None => local_lower = DEFAULT_LOWER,
            Some(forced_lower) => local_lower = forced_lower,
        };
        let data_map = current_state.get_data();
        data_map.iter().for_each(|(device,value_map)|{
            let mut value_list:Vec<&OrderedFloat<f32>> = value_map.keys().collect();
            value_list.sort();
            let mut iteration_count = 0;
            let mut sum = 0;
            let mut pass_iteration_count = 0;
            let saved_data = &mut self.file;
            value_map.iter().for_each(|(value,count)|{
                iteration_count += count;
                if value.into_inner() > local_lower && value.into_inner() < local_upper{
                    pass_iteration_count += count;
                }
                sum += (value.into_inner() * *count as f32) as u128;
                saved_data.with_section(Some(&(device.to_owned() + " read value counts").to_string())).set(&value.to_string(),&count.to_string());
            });

            saved_data.with_section(Some(device))
                .set(ITERATION_COUNT, iteration_count.to_string())
                .set(PASS_COUNT,pass_iteration_count.to_string());
        });
        _ = self.file.write_to_file(self.filename.clone());
    }
}
