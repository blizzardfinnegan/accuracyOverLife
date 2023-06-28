use derivative::Derivative;
use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Eq,Derivative,Debug)]
#[derivative(PartialEq, Hash)]
pub enum Command{
}
pub const COMMAND_MAP:Lazy<HashMap<Command,&str>> = Lazy::new(||HashMap::from([
]));
