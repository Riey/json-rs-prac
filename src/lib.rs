use std::collections::HashMap;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_while, take_while_m_n};
use nom::character::complete::{char, none_of, one_of};
use nom::combinator::map;
use nom::error::{context, ParseError};
use nom::multi::{many0, separated_list};
use nom::number::complete::float;
use nom::sequence::{delimited, preceded, separated_pair, terminated};
use nom::{AsChar, IResult, InputTakeAtPosition};
use std::convert::TryInto;

#[derive(PartialEq, Debug, Clone)]
pub enum Value {
    Null,
    Boolean(bool),
    Number(f32),
    String(String),
    Object(HashMap<String, Value>),
    Array(Vec<Value>),
}

fn null(i: &str) -> IResult<&str, Value> {
    map(tag("null"), |_| Value::Null)(i)
}

fn boolean(i: &str) -> IResult<&str, Value> {
    alt((
        map(tag("true"), |_| Value::Boolean(true)),
        map(tag("false"), |_| Value::Boolean(false)),
    ))(i)
}

fn number(i: &str) -> IResult<&str, Value> {
    map(float, Value::Number)(i)
}

fn unescape(c: char) -> char {
    match c {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        _ => c,
    }
}

fn simple_escape_char(i: &str) -> IResult<&str, char> {
    map(one_of("\"\\nrt"), unescape)(i)
}

fn hex(i: &str) -> IResult<&str, u32> {
    map(take_while_m_n(4, 4, char::is_hex_digit), |hex| {
        u32::from_str_radix(hex, 16).unwrap()
    })(i)
}

fn hex_escape_char(i: &str) -> IResult<&str, char> {
    preceded(char('u'), map(hex, |hex| hex.try_into().unwrap()))(i)
}

fn escape_char(i: &str) -> IResult<&str, char> {
    preceded(char('\\'), alt((hex_escape_char, simple_escape_char)))(i)
}

fn normal_char(i: &str) -> IResult<&str, char> {
    none_of("\\\"")(i)
}

fn js_string(i: &str) -> IResult<&str, String> {
    map(
        delimited(char('"'), many0(alt((escape_char, normal_char))), char('"')),
        |chars| chars.into_iter().collect(),
    )(i)
}

fn string(i: &str) -> IResult<&str, Value> {
    map(js_string, Value::String)(i)
}

fn array(i: &str) -> IResult<&str, Value> {
    map(
        delimited(char('['), separated_list(char(','), value), char(']')),
        Value::Array,
    )(i)
}

fn js_spaces<I: Clone + InputTakeAtPosition, E: ParseError<I>>(i: I) -> IResult<I, I, E>
where
    I::Item: Clone + AsChar,
{
    take_while(|c: I::Item| match c.as_char() {
        ' ' | '\n' | '\r' | '\u{0009}' => true,
        _ => false,
    })(i)
}

#[inline]
fn ws<I: Clone + InputTakeAtPosition, O, E: ParseError<I>>(
    f: impl Fn(I) -> IResult<I, O, E>,
) -> impl Fn(I) -> IResult<I, O, E>
where
    I::Item: Clone + AsChar,
{
    terminated(f, js_spaces)
}

fn object(i: &str) -> IResult<&str, Value> {
    context(
        "object",
        map(
            delimited(
                ws(char('{')),
                separated_list(
                    ws(char(',')),
                    context(
                        "object item",
                        separated_pair(ws(js_string), ws(char(':')), value),
                    ),
                ),
                char('}'),
            ),
            |kv| Value::Object(kv.into_iter().collect()),
        ),
    )(i)
}

fn value_inner(i: &str) -> IResult<&str, Value> {
    alt((null, boolean, number, string, array, object))(i)
}

pub fn value(i: &str) -> IResult<&str, Value> {
    delimited(js_spaces, value_inner, js_spaces)(i)
}

#[test]
fn string_test() {
    let (left, value) = string("\"abd\\tbc\"foo").unwrap();
    assert_eq!(left, "foo");
    assert_eq!(value, Value::String("abd\tbc".into()));
}

#[test]
fn value_test() {
    let (left, value) = value(r#" { "abc" : "def", "foo": ["bar", 123] } "#).unwrap();
    assert!(left.is_empty());
    assert_eq!(
        value,
        Value::Object(
            [
                ("abc".into(), Value::String("def".into())),
                (
                    "foo".into(),
                    Value::Array(vec![Value::String("bar".into()), Value::Number(123.0),])
                )
            ]
            .iter()
            .cloned()
            .collect()
        )
    );
}

#[test]
fn new_line_value_test() {
    let (_, value) = value(
        "
    {
    \"glossary\": 123
}

    ",
    )
    .unwrap();

    assert_eq!(
        value,
        Value::Object(
            [("glossary".into(), Value::Number(123.0))]
                .iter()
                .cloned()
                .collect()
        )
    );
}
