use crate::{
    hardware::*,
    parse_utils::{is_not0, non_comment_lines, AndThenConsuming, IResult},
};

use hashbrown::HashMap;
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{alphanumeric1, char, i16, space0},
    combinator::{cut, map, recognize, success, value},
    error::{ParseError, VerboseError},
    multi::{many1, many1_count},
    sequence::{delimited, preceded, terminated, tuple},
    Parser,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssemblyInstruction {
    Instruction(Instruction),
    Label(String),
    AtIdentifierInstruction(String),
    AtNumberInstruction(i16),
}

fn parse_identifier(input: &str) -> IResult<&str, &str> {
    recognize(many1_count(alt((alphanumeric1, tag("_"), tag(".")))))(input)
}

fn create_c_instruction(args: (DestinationRegisters, u16, JumpCondition)) -> AssemblyInstruction {
    AssemblyInstruction::Instruction(Instruction::create(args.0, args.1, args.2))
}

fn parse_at_number_instruction(input: &str) -> IResult<&str, AssemblyInstruction> {
    let (remainder, number) = i16(input)?;

    if number < 0 {
        return Err(nom::Err::Error(VerboseError::from_error_kind(
            input,
            nom::error::ErrorKind::Digit,
        )));
    }

    Ok((remainder, AssemblyInstruction::AtNumberInstruction(number)))
}

fn parse_at_identifier_instruction(input: &str) -> IResult<&str, AssemblyInstruction> {
    let (remainder, identifier) = parse_identifier(input)?;

    Ok((
        remainder,
        AssemblyInstruction::AtIdentifierInstruction(identifier.to_string()),
    ))
}

fn parse_label(input: &str) -> IResult<&str, AssemblyInstruction> {
    let (remainder, identifier) = parse_identifier(input)?;

    Ok((
        remainder,
        AssemblyInstruction::Label(identifier.to_string()),
    ))
}

fn parse_destination_registers(input: &str) -> IResult<&str, DestinationRegisters> {
    alt((
        terminated(is_not0("="), char('='))
            .and_then_consuming(cut(terminated(
                recognize(many1(alt((tag("A"), tag("M"), tag("D"))))),
                space0,
            )))
            .and_then_consuming(create_destination),
        success(DestinationRegisters::NoDestination),
    ))(input)
}

fn parse_calculation(input: &str) -> IResult<&str, u16> {
    is_not(";/")
        .and_then_consuming(delimited(
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
        ))
        .parse(input)
}

fn parse_jump_condition(input: &str) -> IResult<&str, JumpCondition> {
    alt((
        preceded(
            char(';'),
            preceded(
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
            ),
        ),
        success(JumpCondition::NoJump),
    ))(input)
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
    let mut expected_iter = expected.chars();
    let mut expected_char = expected_iter.next();
    for actual_char in actual.chars() {
        if actual_char.is_whitespace() {
            continue;
        }
        if Some(actual_char) != expected_char {
            return CompareResult::Error;
        }
        expected_char = expected_iter.next();
    }
    if expected_char.is_none() {
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

fn create_destination(args: &str) -> IResult<&str, DestinationRegisters> {
    use DestinationRegisters::*;
    let a_count = args.chars().filter(|&c| c == 'A').count();
    let m_count = args.chars().filter(|&c| c == 'M').count();
    let d_count = args.chars().filter(|&c| c == 'D').count();

    match (a_count, m_count, d_count) {
        (1, 0, 0) => Ok(("", A)),
        (1, 1, 0) => Ok(("", AM)),
        (1, 1, 1) => Ok(("", AMD)),
        (0, 1, 0) => Ok(("", M)),
        (0, 1, 1) => Ok(("", MD)),
        (0, 0, 1) => Ok(("", D)),
        (1, 0, 1) => Ok(("", AD)),
        _ => Err(nom::Err::Error(VerboseError::from_error_kind(
            args,
            nom::error::ErrorKind::Fail,
        ))),
    }
}

fn c_instruction(input: &str) -> IResult<&str, AssemblyInstruction> {
    map(
        tuple((
            parse_destination_registers,
            parse_calculation,
            parse_jump_condition,
        )),
        create_c_instruction,
    )(input)
}

fn a_instruction(input: &str) -> IResult<&str, AssemblyInstruction> {
    preceded(
        tag("@"),
        alt((parse_at_number_instruction, parse_at_identifier_instruction)),
    )(input)
}

fn create_label(input: &str) -> IResult<&str, AssemblyInstruction> {
    delimited(tag("("), parse_label, tag(")"))(input)
}

pub fn instruction(input: &str) -> IResult<&str, AssemblyInstruction> {
    alt((c_instruction, a_instruction, create_label))(input)
}

pub fn parse_instructions(input: &str) -> IResult<&str, Vec<AssemblyInstruction>> {
    non_comment_lines(instruction)(input)
}

pub fn assemble_hack_file(input: &str) -> IResult<&str, Vec<Instruction>> {
    map(parse_instructions, |v| assemble(&v))(input)
}

pub fn assemble(assembly_instructions: &[AssemblyInstruction]) -> Vec<Instruction> {
    let mut at_identifier_map: HashMap<&str, i16> = HashMap::from([
        ("R0", 0),
        ("R1", 1),
        ("R2", 2),
        ("R3", 3),
        ("R4", 4),
        ("R5", 5),
        ("R6", 6),
        ("R7", 7),
        ("R8", 8),
        ("R9", 9),
        ("R10", 10),
        ("R11", 11),
        ("R12", 12),
        ("R13", 13),
        ("R14", 14),
        ("R15", 15),
        ("SP", 0),
        ("LCL", 1),
        ("ARG", 2),
        ("THIS", 3),
        ("THAT", 4),
        ("SCREEN", RAM::SCREEN),
        ("SCREEN", RAM::KBD),
    ]);

    let mut index = 0;
    for assembly_instruction in assembly_instructions.iter() {
        let AssemblyInstruction::Label(label) = assembly_instruction else {
            index += 1;
            continue;
        };
        if at_identifier_map.contains_key(label.as_str()) {
            panic!("already encountered label {label}");
        }

        at_identifier_map.insert(label.as_str(), index);
    }

    let mut rom: Vec<Instruction> = vec![];

    let mut static_var_index = 16;
    for assembly_instruction in assembly_instructions.iter() {
        match assembly_instruction {
            AssemblyInstruction::Instruction(instruction) => rom.push(*instruction),
            AssemblyInstruction::Label(_) => {}
            AssemblyInstruction::AtIdentifierInstruction(identifier) => {
                if !at_identifier_map.contains_key(identifier.as_str()) {
                    at_identifier_map.insert(identifier.as_str(), static_var_index);
                    static_var_index += 1;
                }
                rom.push(Instruction::new(
                    at_identifier_map[identifier.as_str()] as u16,
                ));
            }
            AssemblyInstruction::AtNumberInstruction(value) => {
                rom.push(Instruction::new(*value as u16))
            }
        }
    }

    rom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration() {
        let program = r#"
        // some comments
        @16384
        D=A
        @16 // inline comments
        M=D-1
        @17
        M=0
        @24576
        D=M
        @12
        D;JEQ
        @17
        M=-1
        @17
        D=M
        @16
        AM=M+1
        M=D
        @24576
        D=A-1
        @16
        D=D-M
        @4
        D;JGE
        @16384
        D=A
        @16
        M=D-1
        @4
        0;JMP"#;

        let expected_program: Vec<_> = [
            16384, 60432, 16, 58248, 17, 60040, 24576, 64528, 12, 58114, 17, 61064, 17, 64528, 16,
            65000, 58120, 24576, 60560, 16, 62672, 4, 58115, 16384, 60432, 16, 58248, 4, 60039,
        ]
        .iter()
        .map(|raw| Instruction::new(*raw))
        .collect();

        assert_eq!(assemble_hack_file(program), Ok(("", expected_program)))
    }

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
    fn test_parse_destination() {
        assert!(parse_destination_registers("=").is_err(),);
    }

    #[test]
    fn test_at_number() {
        assert_eq!(
            instruction("@1337"),
            Ok(("", AssemblyInstruction::AtNumberInstruction(1337)))
        );
    }

    #[test]
    fn test_at_identifier() {
        assert_eq!(
            instruction("@Bob123"),
            Ok((
                "",
                AssemblyInstruction::AtIdentifierInstruction("Bob123".to_string())
            ))
        );
    }

    #[test]
    fn test_at_label() {
        assert_eq!(
            instruction("(Bob321)"),
            Ok(("", AssemblyInstruction::Label("Bob321".to_string())))
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

    #[test]
    fn test_duplicate_valid_target() {
        assert!(instruction("AAA=D;JEQ").is_err());
    }

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
