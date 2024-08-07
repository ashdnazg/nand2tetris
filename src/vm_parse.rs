use crate::{
    hardware::Word,
    parse_utils::{non_comment_lines, IResult, ParsableWord},
    vm::*,
};

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alphanumeric1, space1},
    combinator::{map, recognize, value},
    multi::many1_count,
    sequence::{pair, preceded, separated_pair},
};

fn pop_segment(input: &str) -> IResult<&str, PopSegment> {
    alt((
        value(PopSegment::Static, tag("static")),
        value(PopSegment::Local, tag("local")),
        value(PopSegment::Argument, tag("argument")),
        value(PopSegment::This, tag("this")),
        value(PopSegment::That, tag("that")),
        value(PopSegment::Temp, tag("temp")),
        value(PopSegment::Pointer, tag("pointer")),
    ))(input)
}

fn push_segment(input: &str) -> IResult<&str, PushSegment> {
    alt((
        value(PushSegment::Constant, tag("constant")),
        value(PushSegment::Static, tag("static")),
        value(PushSegment::Local, tag("local")),
        value(PushSegment::Argument, tag("argument")),
        value(PushSegment::This, tag("this")),
        value(PushSegment::That, tag("that")),
        value(PushSegment::Temp, tag("temp")),
        value(PushSegment::Pointer, tag("pointer")),
    ))(input)
}

fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(many1_count(alt((alphanumeric1, tag("_"), tag(".")))))(input)
}

fn create_pop(args: (PopSegment, Word)) -> VMCommand {
    VMCommand::Pop {
        segment: args.0,
        offset: args.1,
    }
}

fn create_push(args: (PushSegment, Word)) -> VMCommand {
    VMCommand::Push {
        segment: args.0,
        offset: args.1,
    }
}

fn create_if_goto(label_name: &str) -> VMCommand {
    VMCommand::IfGoto {
        label_name: label_name.to_owned(),
    }
}

fn create_goto(label_name: &str) -> VMCommand {
    VMCommand::Goto {
        label_name: label_name.to_owned(),
    }
}

fn create_label(name: &str) -> VMCommand {
    VMCommand::Label {
        name: name.to_owned(),
    }
}

fn create_function(args: (&str, Word)) -> VMCommand {
    VMCommand::Function {
        name: args.0.to_owned(),
        local_var_count: args.1,
    }
}

fn create_call(args: (&str, Word)) -> VMCommand {
    VMCommand::Call {
        function_name: args.0.to_owned(),
        argument_count: args.1,
    }
}

fn command_two_args<'a, A>(
    keyword: &'a str,
    arg1_parser: impl FnMut(&'a str) -> IResult<&'a str, A>,
) -> impl FnMut(&'a str) -> IResult<&'a str, (A, Word)> {
    preceded(
        pair(tag(keyword), space1),
        separated_pair(arg1_parser, space1, ParsableWord::parse_word),
    )
}

fn command_one_arg<'a>(keyword: &'a str) -> impl FnMut(&'a str) -> IResult<&'a str, &'a str> {
    preceded(pair(tag(keyword), space1), identifier)
}

fn command(input: &str) -> IResult<&str, VMCommand> {
    alt((
        map(command_two_args("push", push_segment), create_push),
        map(command_two_args("pop", pop_segment), create_pop),
        map(command_one_arg("label"), create_label),
        map(command_one_arg("goto"), create_goto),
        map(command_one_arg("if-goto"), create_if_goto),
        map(command_two_args("function", identifier), create_function),
        map(command_two_args("call", identifier), create_call),
        value(VMCommand::Add, tag("add")),
        value(VMCommand::Sub, tag("sub")),
        value(VMCommand::Neg, tag("neg")),
        value(VMCommand::Eq, tag("eq")),
        value(VMCommand::Gt, tag("gt")),
        value(VMCommand::Lt, tag("lt")),
        value(VMCommand::And, tag("and")),
        value(VMCommand::Or, tag("or")),
        value(VMCommand::Not, tag("not")),
        value(VMCommand::Return, tag("return")),
    ))(input)
}

pub fn parse_commands(input: &str) -> IResult<&str, Vec<VMCommand>> {
    non_comment_lines(command)(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_push() {
        assert_eq!(
            command("push argument 0"),
            Ok((
                "",
                VMCommand::Push {
                    segment: PushSegment::Argument,
                    offset: 0
                }
            ))
        );
    }

    #[test]
    fn test_read_pop() {
        assert_eq!(
            command("pop local 1337"),
            Ok((
                "",
                VMCommand::Pop {
                    segment: PopSegment::Local,
                    offset: 1337
                }
            ))
        );
    }

    #[test]
    fn test_read_label() {
        assert_eq!(
            command("label Array.bob"),
            Ok((
                "",
                VMCommand::Label {
                    name: "Array.bob".to_owned()
                }
            ))
        );
    }

    #[test]
    fn test_read_function() {
        assert_eq!(
            command("function Array.bob 1337"),
            Ok((
                "",
                VMCommand::Function {
                    name: "Array.bob".to_owned(),
                    local_var_count: 1337
                }
            ))
        );
    }

    #[test]
    fn test_integration() {
        let code = r#"
        function Array.new 0
        push argument 0
        push constant 0
        gt
        not
        if-goto IF_TRUE0
        // Comment
        //

        goto IF_FALSE0
        label IF_TRUE0
        push constant 2 // inline comment
        call Sys.error 1
        pop temp 0
        label IF_FALSE0
        push argument 0
        call Memory.alloc 1
        return

        function Array.dispose 0
        push argument 0
        pop pointer 0
        push pointer 0

        // Another comment
        call Memory.deAlloc 1
        pop temp 0
        push constant 0
        return               "#;
        let result = parse_commands(code);

        assert_eq!(
            result,
            Ok((
                "",
                vec![
                    VMCommand::Function {
                        name: "Array.new".to_owned(),
                        local_var_count: 0
                    },
                    VMCommand::Push {
                        segment: PushSegment::Argument,
                        offset: 0
                    },
                    VMCommand::Push {
                        segment: PushSegment::Constant,
                        offset: 0
                    },
                    VMCommand::Gt,
                    VMCommand::Not,
                    VMCommand::IfGoto {
                        label_name: "IF_TRUE0".to_owned()
                    },
                    VMCommand::Goto {
                        label_name: "IF_FALSE0".to_owned()
                    },
                    VMCommand::Label {
                        name: "IF_TRUE0".to_owned()
                    },
                    VMCommand::Push {
                        segment: PushSegment::Constant,
                        offset: 2
                    },
                    VMCommand::Call {
                        function_name: "Sys.error".to_owned(),
                        argument_count: 1
                    },
                    VMCommand::Pop {
                        segment: PopSegment::Temp,
                        offset: 0
                    },
                    VMCommand::Label {
                        name: "IF_FALSE0".to_owned()
                    },
                    VMCommand::Push {
                        segment: PushSegment::Argument,
                        offset: 0
                    },
                    VMCommand::Call {
                        function_name: "Memory.alloc".to_owned(),
                        argument_count: 1
                    },
                    VMCommand::Return,
                    VMCommand::Function {
                        name: "Array.dispose".to_owned(),
                        local_var_count: 0
                    },
                    VMCommand::Push {
                        segment: PushSegment::Argument,
                        offset: 0
                    },
                    VMCommand::Pop {
                        segment: PopSegment::Pointer,
                        offset: 0
                    },
                    VMCommand::Push {
                        segment: PushSegment::Pointer,
                        offset: 0
                    },
                    VMCommand::Call {
                        function_name: "Memory.deAlloc".to_owned(),
                        argument_count: 1
                    },
                    VMCommand::Pop {
                        segment: PopSegment::Temp,
                        offset: 0
                    },
                    VMCommand::Push {
                        segment: PushSegment::Constant,
                        offset: 0
                    },
                    VMCommand::Return
                ]
            ))
        );
    }
}
