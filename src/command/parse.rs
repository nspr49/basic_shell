use std::sync::{Arc, Mutex};

use regex::Regex;

use super::command::cmd::{Command, CommandList, NoCommand, ProcessStatus, SimpleCommand};

pub fn parse(cmd_argument: &mut String) -> Command {
    cmd_argument.pop();
    let basic_cmd = r"[a-zA-Z-]+(?:\s+[a-zA-Z-]+)*";
    let basic_background = Regex::new(&format!("^{}[ ]+&[ ]*$", basic_cmd)).unwrap();
    let and_regex =
        Regex::new(&format! {"^({}[ ]*[&][&][ ]*)+{}[ ]*$",basic_cmd, basic_cmd}).unwrap();
    let pipe_regex =
        Regex::new(&format! {"^({}[ ]*[|][ ]*)+{}[ ]*$",basic_cmd, basic_cmd}).unwrap();
    let or_regex =
        Regex::new(&format! {"^({}[ ]*[|][|][ ]*)+{}[ ]*$",basic_cmd, basic_cmd}).unwrap();

    if basic_background.is_match(cmd_argument) {
        let mut cmd: Vec<&str> = cmd_argument.split_whitespace().collect();
        cmd.pop().unwrap();
        let res = extract_simple(&cmd);
        match res.0 {
            Some(command) => Command::SimpleCommand(SimpleCommand {
                command: String::from(command),
                args: res.1.to_owned(),
                background: true,
            }),
            None => Command::NoCommand,
        }
    } else if and_regex.is_match(cmd_argument) {
        let list = get_command_list(cmd_argument, "&&");
        return Command::CommandList(CommandList {
            commands: list,
            kind: super::command::cmd::CommandListType::AND,
        });
    } else if pipe_regex.is_match(cmd_argument) {
        let list = get_command_list(cmd_argument, "|");
        return Command::CommandList(CommandList {
            commands: list,
            kind: super::command::cmd::CommandListType::PIPE,
        });
    } else if or_regex.is_match(cmd_argument) {
        let list = get_command_list(cmd_argument, "||");
        return Command::CommandList(CommandList {
            commands: list,
            kind: super::command::cmd::CommandListType::OR,
        });
    } else {
        let cmd: Vec<&str> = cmd_argument.split_whitespace().collect();
        let res = extract_simple(&cmd);
        return match res.0 {
            Some(command) => Command::SimpleCommand(SimpleCommand {
                command: String::from(command),
                args: res.1.to_owned(),
                background: false,
            }),
            None => Command::NoCommand,
        };
    }
}

fn get_command_list(command_string: &str, separator: &str) -> Vec<SimpleCommand> {
    let vals: Vec<&str> = command_string.split(separator).collect();
    let mut cmd_list: Vec<SimpleCommand> = Vec::new();
    for val in vals {
        let single_command: Vec<&str> = val.split_whitespace().collect();
        let res = extract_simple(&single_command);
        if let Some(command) = res.0 {
            cmd_list.push(SimpleCommand {
                command: String::from(command),
                args: res.1.to_owned(),
                background: false,
            })
        };
    }
    cmd_list
}

fn extract_simple<'a>(arguments: &'a Vec<&str>) -> (Option<&'a str>, Vec<String>) {
    let cmd = if arguments.get(0).is_some() {
        Some(arguments.get(0).unwrap().clone())
    } else {
        None
    };
    let args: Vec<String>;
    if arguments.len() < 1 {
        args = Vec::new();
    } else {
        args = arguments.iter().map(|s| String::from(*s)).collect();
    }
    (cmd, args)
}
