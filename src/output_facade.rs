use std::{collections::HashMap, sync::Mutex};
use ini::Ini;
use chrono::Local;
use ordered_float::OrderedFloat;

const ITERATION_COUNT:&str="iterations completed this run";
const PASS_COUNT:&str="passing iterations";
const STD_DEV:&str="Standard deviation";
const MEDIAN:&str="Median";
const DEFAULT_LOWER:f64=35.8;
const DEFAULT_UPPER:f64=36.2;
pub struct OutputFile{
    file:Ini,
    filename:String,
}

pub struct TestState{
    data_map: Mutex<HashMap<String,HashMap<OrderedFloat<f64>,u64>>>
}

impl TestState{
    pub fn new(device_names:Vec<String>) -> Self{
        let output = Self{
            data_map:Mutex::new(HashMap::new())
        };
        device_names.iter()
            .for_each(|device| _ = output.data_map.lock().unwrap().insert(device.to_string(),HashMap::new()));
        return output;
    }

    pub fn add_iteration(&self,device_name:String, value:f64){
        let mut all_data = self.data_map.lock().unwrap();
        if !all_data.contains_key(&device_name){
            all_data.insert(device_name.to_string(),HashMap::new());
        }
        //Device object should be created at this point, unwrap is safe
        let device_data = all_data.get_mut(&device_name).unwrap();
        if !device_data.contains_key(&value.into()){
            device_data.insert(OrderedFloat(value), 1);
        }
        //Value object should be created at this point, unwrap is safe
        else{
            let stored_value = device_data.get_mut(&value.into()).unwrap();
            *stored_value += 1;
        }
    }

    pub fn get_data(&self) -> HashMap<String,HashMap<OrderedFloat<f64>,u64>>{
        self.data_map.lock().unwrap().clone()
    }
}

impl OutputFile{
    pub fn new(device_names:Vec<String>) -> Self{
        let mut config = Ini::new();
        for name in device_names{
            config.with_section(Some(name))
                  .set(ITERATION_COUNT,0.to_string())
                  .set(PASS_COUNT,0.to_string())
                  .set(STD_DEV,0.to_string())
                  .set(MEDIAN, 0.to_string());
        };
        let mut filename = String::from("output/");
        filename.push_str(&Local::now().to_rfc3339());
        _ = config.write_to_file(filename);
        Self{
            file:config,
            filename:String::from(Local::now().to_rfc3339()),
        }
    }
    
    pub fn write_values(&mut self, current_state:&TestState,mut upper_bound:Option<f64>,mut lower_bound:Option<f64>){
        if upper_bound == None{
            upper_bound = Some(DEFAULT_UPPER);
        }
        if lower_bound == None{
            lower_bound = Some(DEFAULT_LOWER);
        }
        let data_map = current_state.get_data();
        data_map.iter().for_each(|(device,value_map)|{
            let mut value_list:Vec<&OrderedFloat<f64>> = value_map.keys().collect();
            value_list.sort();
            let mut index_list:Vec<u64> = Vec::new();
            let mut iteration_count = 0;
            let mut sum = 0;
            let mut std_dev_intermediate = 0.0;
            let mut median = 0.0;
            let mut pass_iteration_count = 0;
            value_map.iter().for_each(|(value,count)|{
                iteration_count += count;
                //                          unwraps are required and safe here
                if value.into_inner() > lower_bound.unwrap() && value.into_inner() < upper_bound.unwrap(){
                    pass_iteration_count += count;
                }
                sum += (value.into_inner() * *count as f64) as u128;
            });
            let mut counter = 0;
            for value in &value_list{
                //Value list is created from keys in map, unwrap is safe
                counter += value_map.get(value).unwrap();
                index_list.push(counter.clone());
            }
            let mean = sum as f64 / iteration_count as f64;
            let median_index = (iteration_count + 1) / 2;
            if value_list.len() < 2{
                median = value_list[0].into_inner();
            }
            else{
                for i in 0..index_list.len(){
                    let index_guess = index_list[i];
                    if index_guess >= median_index{
                        if iteration_count % 2 == 0 || index_guess > (median_index + 1){
                            median = value_list[i-1].into_inner();
                        }
                        else{
                            median = (value_list[i+1].into_inner() + value_list[i].into_inner()) / 2.0;
                        }
                    }
                }
            }

            value_map.iter().for_each(|(value,count)|{
                for _ in 0..*count{
                    std_dev_intermediate += (value - mean).powi(2);
                }
            });

            let std_dev_final = (std_dev_intermediate / iteration_count as f64).sqrt();
            let saved_data = &mut self.file;
            saved_data.with_section(Some(device))
                .set(ITERATION_COUNT, iteration_count.to_string())
                .set(PASS_COUNT,pass_iteration_count.to_string())
                .set(STD_DEV,std_dev_final.to_string())
                .set(MEDIAN,median.to_string());
        });
        _ = self.file.write_to_file(self.filename.clone());
    }
}
