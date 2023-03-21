use crate::hardware::*;

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{alphanumeric1, i16, line_ending, space0, space1},
    combinator::{all_consuming, eof, map, opt, recognize, value},
    error::{ParseError, VerboseError},
    multi::{many1, many1_count, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    Compare,
};

type IResult<I, O> = nom::IResult<I, O, VerboseError<I>>;

#[derive(PartialEq, Eq, Debug)]
enum AssemblyInstruction {
    Instruction(Instruction),
    Label(String),
    AtInstruction(String),
}

fn second_pass(_: &[AssemblyInstruction]) -> Vec<Instruction> {
    vec![]
}

fn create_c_instruction(
    args: (DestinationRegisters, u16, Option<JumpCondition>),
) -> AssemblyInstruction {
    AssemblyInstruction::Instruction(
        Instruction::create(args.0, args.1, args.2.unwrap_or(JumpCondition::NoJump)).unwrap(),
    )
}

fn parse_destination_registers<'a>() -> impl FnMut(&'a str) -> IResult<&'a str, DestinationRegisters>
{
    map(
        opt(terminated(
            delimited(
                space0,
                recognize(many1(alt((tag("A"), tag("M"), tag("D"))))),
                space0,
            ),
            tag("="),
        )),
        create_destination,
    )
}

fn parse_calculation<'a>() -> impl FnMut(&'a str) -> IResult<&'a str, u16> {
    delimited(
        space0,
        alt((
            alt((
                value(0x01AA, tag_no_whitespace("0")),
                value(0x01BF, tag_no_whitespace("1")),
                value(0x01BA, tag_no_whitespace("-1")),
                value(0x018C, tag_no_whitespace("D")),
                value(0x01B0, tag_no_whitespace("A")),
                value(0x01F0, tag_no_whitespace("M")),
                value(0x018D, tag_no_whitespace("!D")),
                value(0x01B1, tag_no_whitespace("!A")),
                value(0x01F1, tag_no_whitespace("!M")),
                value(0x018F, tag_no_whitespace("-D")),
                value(0x01B3, tag_no_whitespace("-A")),
                value(0x01F3, tag_no_whitespace("-M")),
                value(0x019F, tag_no_whitespace("D+1")),
                value(0x01B7, tag_no_whitespace("A+1")),
                value(0x01F7, tag_no_whitespace("M+1")),
                value(0x018E, tag_no_whitespace("D-1")),
                value(0x01B2, tag_no_whitespace("A-1")),
                value(0x01F2, tag_no_whitespace("M-1")),
                value(0x0182, tag_no_whitespace("D+A")),
                value(0x0182, tag_no_whitespace("A+D")),
                value(0x01C2, tag_no_whitespace("D+M")),
            )),
            alt((
                value(0x01C2, tag_no_whitespace("M+D")),
                value(0x0193, tag_no_whitespace("D-A")),
                value(0x0187, tag_no_whitespace("A-D")),
                value(0x01D3, tag_no_whitespace("D-M")),
                value(0x01C7, tag_no_whitespace("M-D")),
                value(0x0180, tag_no_whitespace("D&A")),
                value(0x0180, tag_no_whitespace("A&D")),
                value(0x01C0, tag_no_whitespace("D&M")),
                value(0x01C0, tag_no_whitespace("M&D")),
                value(0x0195, tag_no_whitespace("D|A")),
                value(0x0195, tag_no_whitespace("A|D")),
                value(0x01D5, tag_no_whitespace("D|M")),
                value(0x01D5, tag_no_whitespace("M|D")),
            )),
        )),
        space0,
    )
}

fn parse_jump_condition<'a>() -> impl FnMut(&'a str) -> IResult<&'a str, Option<JumpCondition>> {
    opt(preceded(
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
            space0,
        ),
    ))
}

#[derive(PartialEq, Eq, Debug)]
enum CompareResult {
    /// Comparison was successful
    Ok,
    /// We need more data to be sure
    Incomplete,
    /// Comparison failed
    Error,
}

fn compare_no_whitespace(actual: &str, expected: &str) -> CompareResult {
    let mut actual_iter = actual.chars();
    let mut expected_iter = expected.chars();
    let mut expected_char = expected_iter.next();
    while let Some(actual_char) = actual_iter.next() {
        if actual_char.is_whitespace() {
            continue;
        }
        if Some(actual_char) != expected_char {
            return CompareResult::Error;
        }
        expected_char = expected_iter.next();
    }
    if expected_char == None {
        CompareResult::Ok
    } else {
        CompareResult::Incomplete
    }
}

pub fn tag_no_whitespace<'a>(tag: &'a str) -> impl Fn(&'a str) -> IResult<&'a str, &'a str> {
    move |i: &str| {
        let res: IResult<_, _> = match compare_no_whitespace(i, tag) {
            CompareResult::Ok => Ok(("", i)),
            _ => {
                let e: nom::error::ErrorKind = nom::error::ErrorKind::Tag;
                Err(nom::Err::Error(VerboseError::from_error_kind(i, e)))
            }
        };
        res
    }
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
        alt((
            map(
                tuple((
                    parse_destination_registers(),
                    parse_calculation(),
                    parse_jump_condition(),
                )),
                create_c_instruction,
            ),
        )),
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
            instruction("M = D  -   1   "),
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
        // println!("{:?}", instruction("AE=D;JEQ"));
        assert!(instruction("AE=D;JEQ").is_err());
    }

    // #[test]
    // fn test_duplicate_valid_target() {
    //     assert!(instruction("AAA=D;JEQ").is_err());
    // }

    #[test]
    fn test_compare_no_whitespace() {
        use CompareResult::*;

        assert_eq!(compare_no_whitespace("ab", "abc"), Incomplete);
        assert_eq!(compare_no_whitespace("abc", "abd"), Error);
        assert_eq!(compare_no_whitespace("", "def"), Incomplete);

        assert_eq!(compare_no_whitespace("a b c", "def"), Error);
        assert_eq!(compare_no_whitespace("a  b", "ab"), Ok);
        assert_eq!(compare_no_whitespace("a  b", "abc"), Incomplete);
        assert_eq!(compare_no_whitespace("a b   c", "abd"), Error);
        assert_eq!(compare_no_whitespace("  ", "def"), Incomplete);
    }
}
