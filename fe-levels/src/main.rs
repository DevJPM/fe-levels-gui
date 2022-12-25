/*use std::{collections::HashMap, fmt};

use repl_rs::{Command, Parameter, Repl, Value};

mod repl;

#[derive(Debug)]
pub enum Error {
    ReplError(repl_rs::Error),
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    StatNotFound(String),
    NoUnit,
    NoPromotionFound(String)
}

impl From<repl_rs::Error> for Error {
    fn from(error : repl_rs::Error) -> Self { Error::ReplError(error) }
}

impl From<std::io::Error> for Error {
    fn from(error : std::io::Error) -> Self { Error::IoError(error) }
}

impl From<serde_json::Error> for Error {
    fn from(error : serde_json::Error) -> Self { Error::JsonError(error) }
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f : &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::ReplError(error) => write!(f, "{error}"),
            Error::IoError(error) => write!(f, "{error}"),
            Error::JsonError(error) => write!(f, "{error}"),
            Error::StatNotFound(input) => write!(f, "Failed to interpret the stat {input}."),
            Error::NoUnit => write!(f, "There's no unit currently loaded for editing."),
            Error::NoPromotionFound(input) => write!(f, "No promotion found for the input {input}.")
        }
    }
}

type Arguments = HashMap<String, Value>;
type Return = Result<Option<String>, Error>;

trait FeRepl {
    fn new_unit(&mut self, args : Arguments) -> Return;
    fn update_base(&mut self, args : Arguments) -> Return;
    fn update_stat(&mut self, args : Arguments) -> Return;
    fn update_growth(&mut self, args : Arguments) -> Return;
    fn update_cap(&mut self, args : Arguments) -> Return;
    fn new_promotion(&mut self, args : Arguments) -> Return;
    fn add_level(&mut self, args : Arguments) -> Return;
    fn add_promotion(&mut self, args : Arguments) -> Return;
    fn heat_map(&mut self, args : Arguments) -> Return;
    fn save_unit(&mut self, args : Arguments) -> Return;
    fn load_unit(&mut self, args : Arguments) -> Return;
    fn save_progression(&mut self, args : Arguments) -> Return;
    fn load_progression(&mut self, args : Arguments) -> Return;
    fn save_histograms(&mut self, args : Arguments) -> Return;
}

fn exit<C>(_args : HashMap<String, Value>, _context : &mut C) -> Result<Option<String>, Error> {
    std::process::exit(0);
}

macro_rules! callbacker {
    ($name : ident) => {
        |args, context : &mut Box<dyn FeRepl>| context.$name(args)
    };
}

macro_rules! command {
    ($name : ident) => {
        Command::new(stringify!($name), callbacker!($name))
    };
}*/

pub fn main() -> Result<(), Error> {
    Ok(())
    /*
    let context : Box<dyn FeRepl> = Box::new(repl::GbaFe::new("fe8").unwrap());

    Ok(Repl::new(context)
        .use_completion(true)
        .with_description("Tool to assist with fire-emblem level-up questions")
        .with_version("0.1.0")
        .with_name("fe-levels")
        // general management
        //.add_command(Command::new("switch_game", todo!()))
        // specify unit
        .add_command(command!(new_unit).with_parameter(Parameter::new("name").set_required(true)?)?)
        .add_command(
            command!(update_base)
                .with_parameter(Parameter::new("stat").set_required(true)?)?
                .with_parameter(Parameter::new("value").set_required(true)?)?
        )
        .add_command(
            command!(update_stat)
                .with_parameter(Parameter::new("stat").set_required(true)?)?
                .with_parameter(Parameter::new("value").set_required(true)?)?
        )
        .add_command(
            command!(update_growth)
                .with_parameter(Parameter::new("stat").set_required(true)?)?
                .with_parameter(Parameter::new("value").set_required(true)?)?
        )
        .add_command(
            command!(update_cap)
                .with_parameter(Parameter::new("stat").set_required(true)?)?
                .with_parameter(Parameter::new("value").set_required(true)?)?
        )
        // specify promotions
        .add_command(command!(new_promotion))
        // add new unit growth opportunities ("progression")
        .add_command(command!(add_level))
        .add_command(
            command!(add_promotion)
                .with_parameter(Parameter::new("target_class").set_required(true)?)?
        )
        // perform analysis
        .add_command(command!(heat_map))
        // perform data management
        .add_command(command!(save_unit))
        .add_command(
            command!(load_unit).with_parameter(Parameter::new("unit_name").set_required(true)?)?
        )
        .add_command(
            command!(save_progression)
                .with_parameter(Parameter::new("filename").set_required(true)?)?
        )
        .add_command(
            command!(load_progression)
                .with_parameter(Parameter::new("filename").set_required(true)?)?
        )
        .add_command(
            command!(save_histograms)
                .with_parameter(Parameter::new("filename").set_required(true)?)?
                .with_parameter(Parameter::new("reduction").set_required(true)?)?
                .with_parameter(Parameter::new("reduction_param").set_required(true)?)?
        )
        // general stuff
        .add_command(Command::new("exit", exit).with_help("Exits the program."))
        .add_command(Command::new("quit", exit).with_help("Exits the program."))
        .run()?)*/
}
