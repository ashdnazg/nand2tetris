use crate::hardware::*;

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{alphanumeric1, i16, line_ending, space0, space1},
    combinator::{all_consuming, eof, map, opt, recognize, value},
    error::VerboseError,
    multi::{many1_count, separated_list1, many1},
    sequence::{delimited, pair, preceded, separated_pair, tuple, terminated},
};

type IResult<I, O> = nom::IResult<I, O, VerboseError<I>>;

#[derive(PartialEq, Eq, Debug)]
enum AssemblyInstruction {
    Instruction(Instruction),
    Label(String),
    AtInstruction(String),
}

fn secondPass(_: &[AssemblyInstruction]) -> Vec<Instruction> {
    vec![]
}

fn create_instruction(args: (DestinationRegisters, &str, Option<JumpCondition>)) -> AssemblyInstruction {
    AssemblyInstruction::Instruction(Instruction::create(args.0, "D-1", args.2.unwrap_or(JumpCondition::NoJump)).unwrap())
}

fn create_destination(args: Option<&str>) -> DestinationRegisters {
    use DestinationRegisters::*;
    let Some(s) = args else {
        return NoDestination;
    };
    let has_a = s.contains("A");
    let has_m = s.contains("M");
    let has_d = s.contains("D");

    match (has_a, has_m, has_d) {
        (true, false, false) => A,
        (true, true, false) => AM,
        (true, true, true) => AMD,
        (false, true, false) => M,
        (false, true, true) => MD,
        (false, false, true) => D,
        (true, false, true) => AD,
        (false, false, false) => unreachable!("should not be empty"),
    }
}

fn instruction(input: &str) -> IResult<&str, AssemblyInstruction> {
    delimited(
        space0,
        map(
            tuple((
                map(
                    opt(
                        terminated(
                            delimited(
                                space0,
                                recognize(
                                    many1(
                                        alt((
                                            tag("A"),
                                            tag("M"),
                                            tag("D"),
                                        ))
                                    )
                                ),
                                space0
                            ),
                            tag("=")
                        )
                    ),
                    create_destination
                ),
                tag("D-1"),
                opt(
                    preceded(
                        tag(";"),
                        delimited(
                            space0,
                            alt((
                                value(JumpCondition::JEQ, tag("JEQ")),
                                value(JumpCondition::JGE, tag("JGE")),
                                value(JumpCondition::JGT, tag("JGT")),
                                value(JumpCondition::JLE, tag("JLE")),
                                value(JumpCondition::JLT, tag("JLT")),
                                value(JumpCondition::JMP, tag("JMP")),
                                value(JumpCondition::JNE, tag("JNE")),
                            )),
                            space0
                        )
                    )
                ),
            )),
            create_instruction
        ),
        space0,
    )(input)
}

fn non_command_lines(input: &str) -> IResult<&str, ()> {
    value(
        (),
        many1_count(alt((
            recognize(tuple((
                space0,
                tag("//"),
                opt(is_not("\n\r")),
                alt((line_ending, eof)),
            ))),
            recognize(pair(space1, alt((line_ending, eof)))),
            line_ending,
        ))),
    )(input)
}

fn commands(input: &str) -> IResult<&str, Vec<AssemblyInstruction>> {
    all_consuming(delimited(
        opt(non_command_lines),
        separated_list1(non_command_lines, instruction),
        opt(non_command_lines),
    ))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_a() {
        assert_eq!(
            instruction("M=D-1"),
            Ok((
                "",
                AssemblyInstruction::Instruction(Instruction::new(58248))
            ))
        );
    }

    #[test]
    fn test_equal_without_target() {
        assert!(instruction("=D;JEQ").is_err());
    }

    #[test]
    fn test_invalid_target() {
        assert!(instruction("E=D;JEQ").is_err());
    }

    #[test]
    fn test_invalid_and_valid_target() {
        assert!(instruction("AE=D;JEQ").is_err());
    }

    // #[test]
    // fn test_duplicate_valid_target() {
    //     assert!(instruction("AAA=D;JEQ").is_err());
    // }
}
