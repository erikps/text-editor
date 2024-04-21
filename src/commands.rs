use core::panic;

use crate::{
    io::{load, save},
    state::{Editor, State},
};

#[derive(Clone, Debug)]
pub enum CommandParameterType {
    StringParameter,
    IntParameter,
    FloatParameter,
    OptionalParameter(Box<CommandParameterType>),
}

#[derive(Clone, Debug)]
pub enum CommandParameter {
    StringParameter(String),
    IntParameter(i32),
    FloatParameter(f32),
    OptionalParameter(Box<Option<CommandParameter>>),
}

type ExecuteCommand = dyn Fn(Vec<CommandParameter>, &mut Editor) -> bool;

pub struct Command {
    pub names: Vec<String>,
    pub parameters: Vec<CommandParameterType>,
    pub execute: Box<ExecuteCommand>,
}

impl Command {
    pub fn new(
        names: &[&str],
        parameters: &[CommandParameterType],
        execute: Box<ExecuteCommand>,
    ) -> Self {
        Command {
            names: Vec::from_iter(names.iter().map(|s| String::from(*s))),
            parameters: Vec::from(parameters),
            execute,
        }
    }
}

fn parse_parameter(
    parameter_string: &str,
    parameter_type: &CommandParameterType,
) -> Result<CommandParameter, String> {
    let param = match parameter_type {
        CommandParameterType::StringParameter => {
            CommandParameter::StringParameter((*parameter_string).to_owned())
        }
        CommandParameterType::FloatParameter => {
            if let Ok(parsed_float) = parameter_string.parse() {
                CommandParameter::FloatParameter(parsed_float)
            } else {
                return Err("Could not parse float".to_owned());
            }
        }
        CommandParameterType::IntParameter => {
            if let Ok(parsed_int) = parameter_string.parse() {
                CommandParameter::IntParameter(parsed_int)
            } else {
                return Err("Could not parse int".to_owned());
            }
        }
        CommandParameterType::OptionalParameter(optional_type) => {
            parse_parameter(parameter_string, optional_type)?
        }
    };
    Ok(param)
}

pub fn prepare_command(
    commands: &Vec<Command>,
    string: &str,
) -> Result<(Vec<CommandParameter>, usize), String> {
    let no_colon_string = &string[1..];

    let fragments: Vec<&str> = no_colon_string.split(" ").collect();
    if let Some(command_string) = fragments.get(0) {
        let parameter_strings = &fragments[1..];

        for command_index in 0..commands.len() {
            let command = &commands[command_index];
            for name in &command.names {
                if name == *command_string {
                    // found the correct command command to execute
                    let mut parameters: Vec<CommandParameter> = Vec::new();

                    // iterate over expected parameters and construct a list of actual parameters
                    for param_index in 0..command.parameters.len() {
                        let parameter_type = &command.parameters[param_index];
                        if let Some(parameter_string) = parameter_strings.get(param_index) {
                            let parameter = parse_parameter(&parameter_string, parameter_type);
                            if let Ok(parameter) = parameter {
                                parameters.push(parameter);
                            }
                        } else {
                            match parameter_type {
                                CommandParameterType::OptionalParameter(_) => {}
                                _ => {
                                    return Err("Too few parameters provided".to_owned());
                                }
                            }
                        }
                    }

                    return Ok((parameters, command_index));
                }
            }
        }
        return Err(format!("Could not find command \"{}\"", command_string));
    }

    Err("Unexpected Error".to_owned())
}

use CommandParameter::*;
pub fn get_standard_commands() -> Vec<Command> {
    vec![
        Command::new(
            &["w", "write"],
            &[CommandParameterType::OptionalParameter(Box::from(
                CommandParameterType::StringParameter,
            ))],
            Box::from(|params: Vec<CommandParameter>, editor: &mut Editor| {
                if let Some(OptionalParameter(path_parameter)) = params.get(0) {
                    let filepath = path_parameter.clone().and_then(|param| match param {
                        StringParameter(string) => Some(string),
                        _ => None,
                    });
                    let result = save(editor.buffer(), filepath.clone());
                    match result {
                        Ok(_) => true,
                        Err(e) => {
                            false
                        }
                    }
                } else {
                    save(editor.buffer(), None).is_ok()
                }
            }),
        ),
        Command::new(
            &["e", "edit"],
            &[CommandParameterType::StringParameter],
            Box::from(|params: Vec<CommandParameter>, editor: &mut Editor| {
                if let Some(StringParameter(filepath)) = params.get(0) {
                    match load(&filepath) {
                        Ok(rope) => {
                            editor.add_buffer(rope, Some(filepath.clone()));
                            return true;
                        }
                        Err(e) => {
                            println!("{}", e);
                        }
                    }
                }
                false
            }),
        ),
        Command::new(
            &["q", "quit"],
            &[],
            Box::from(|_: Vec<CommandParameter>, _: &mut Editor| {
                std::process::exit(0);
            }),
        ),
        Command::new(
            &["bn", "bnext"],
            &[],
            Box::from(|_: Vec<CommandParameter>, editor: &mut Editor| {
                editor.next_buffer();
                true
            }),
        ),
        Command::new(
            &["bp", "bprevious"],
            &[],
            Box::from(|_: Vec<CommandParameter>, editor: &mut Editor| {
                editor.previous_buffer();
                true
            }),
        ),
    ]
}
