use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{line_ending, not_line_ending, space0},
    combinator::{all_consuming, map, opt, rest, value},
    error::{ParseError, VerboseError},
    multi::separated_list0,
    sequence::{delimited, preceded, terminated},
    FindToken, InputLength, InputTakeAtPosition, Parser,
};


pub type IResult<I, O> = nom::IResult<I, O, VerboseError<I>>;

pub trait AndThenConsuming<I, O, E> {
    fn and_then_consuming<O2, G>(self, g: G) -> impl Parser<I, O2, E>
    where
        I: InputLength,
        G: Parser<O, O2, E>,
        Self: core::marker::Sized;
}

impl<I, O: InputLength, E: ParseError<O>, T: Parser<I, O, E>> AndThenConsuming<I, O, E> for T {
    fn and_then_consuming<O2, G>(self, g: G) -> impl Parser<I, O2, E>
    where
        I: InputLength,
        G: Parser<O, O2, E>,
        Self: core::marker::Sized,
    {
        self.and_then(all_consuming(g))
    }
}

pub fn is_not0<T, Input>(arr: T) -> impl Fn(Input) -> IResult<Input, Input>
where
    Input: InputTakeAtPosition,
    T: FindToken<<Input as InputTakeAtPosition>::Item>,
{
    move |i: Input| i.split_at_position_complete(|c| arr.find_token(c))
}

pub fn strip_comment(input: &str) -> IResult<&str, &str> {
    terminated(is_not0("/"), opt(preceded(tag("//"), rest)))(input)
}

pub fn non_comment_lines<'a, O, F>(line_parser: F) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<O>>
where
    F: FnMut(&'a str) -> IResult<&'a str, O>,
    O: Clone,
{
    all_consuming(map(
        separated_list0(
            line_ending,
            not_line_ending
                .and_then_consuming(strip_comment)
                .and_then_consuming(alt((
                    map(delimited(space0, line_parser, space0), |l| Some(l)),
                    value(None, space0),
                ))),
        ),
        |v| v.into_iter().filter_map(|i| i).collect::<Vec<_>>(),
    ))
}
