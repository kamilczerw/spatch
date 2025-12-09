use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{char, multispace0},
    combinator::{all_consuming, map},
    error::context,
    multi::{separated_list0, separated_list1},
    sequence::{delimited, preceded, separated_pair},
};

use super::{Segment, Spath};

// /foo/bar/baz - allowed - simple path
// foo/bar/bas - not allowed, missing leading `/`
// /foo/ [ id=123 ] /bar - allowed - filter segment
// /foo/ [ id=123, type=active ] /bar - allowed - multiple conditions in filter
// /foo/ [ id=123 /bar - not allowed - missing closing `]` in filter
// /foo//bar - not allowed - empty segment
// /foo/ [ id=123, ] /bar - not allowed - trailing comma in filter
// /foo/ [ =123 ] /bar - not allowed - missing key in condition
// /foo/ [ id= ] /bar - not allowed - missing value in condition
// /foo/ [ id=123 type=active ] /bar - not allowed - missing comma between conditions
// /foo/ [ id=12/3 ] /bar - not allowed - invalid character '/' in value
pub(crate) fn parse_path(input: &str) -> IResult<&str, Spath> {
    let (rest, segments): (&str, Vec<Segment>) = all_consuming(preceded(
        ws(char('/')),
        separated_list0(ws(char('/')), parse_segment),
    ))
    .parse(input)?;

    Ok((rest, Spath { segments }))
}

// helper: wrap a parser and eat optional whitespace around it
fn ws<'a, O, F>(inner: F) -> impl Parser<&'a str, Output = O, Error = nom::error::Error<&'a str>>
where
    F: Parser<&'a str, Output = O, Error = nom::error::Error<&'a str>>,
{
    delimited(multispace0, inner, multispace0)
}

fn parse_segment(input: &str) -> IResult<&str, super::Segment> {
    context(
        "segment",
        alt((parse_filter_segment, ws(parse_key_segment))),
    )
    .parse(input)
}

fn parse_key_segment(input: &str) -> IResult<&str, Segment> {
    // Key: run until '/' or '['
    let is_key_char = |c: char| c != '/' && c != '[';
    map(take_while1(is_key_char), |s: &str| {
        Segment::Field(s.trim().to_string())
    })
    .parse(input)
}

fn parse_filter_segment(input: &str) -> IResult<&str, Segment> {
    map(
        delimited(
            ws(char('[')),
            separated_list1(ws(char(',')), parse_condition),
            ws(char(']')),
        ),
        Segment::Filter,
    )
    .parse(input)
}

fn parse_condition(input: &str) -> IResult<&str, (String, String)> {
    map(
        separated_pair(ws(parse_ident), ws(char('=')), ws(parse_value)),
        |(k, v)| (k.trim().to_string(), v.trim().to_string()),
    )
    .parse(input)
}

// identifier for field names in conditions
fn parse_ident(input: &str) -> IResult<&str, &str> {
    let is_ident_char = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    take_while1(is_ident_char).parse(input)
}

// value inside conditions (simple unescaped token)
fn parse_value(input: &str) -> IResult<&str, &str> {
    // can't contain '&' or ']' because they delimit conditions and filters
    let is_val_char = |c: char| c != ',' && c != ']';
    take_while1(is_val_char).parse(input)
}

#[cfg(test)]
mod tests {
    use assert2::check;

    use super::*;

    #[test]
    fn test_parse_path() {
        let input = "/a/b/[id=foo]/c";
        let result = parse_path(input);
        check!(result.is_ok());
        let (rest, spath) = result.unwrap();
        check!(rest == "");
        check!(spath.segments.len() == 4);
        check!(spath.segments[0] == Segment::Field(String::from("a")));
        check!(spath.segments[1] == Segment::Field(String::from("b")));
        check!(spath.segments[2] == Segment::Filter(vec![("id".to_string(), "foo".to_string())]));
        check!(spath.segments[3] == Segment::Field(String::from("c")));
    }

    #[test]
    fn test_parse_filter_segment() {
        let input = "[key1=val1,key2=val2]";
        let result = parse_filter_segment(input);
        check!(result.is_ok());
        let (rest, segment) = result.unwrap();
        check!(rest == "");
        check!(
            segment
                == Segment::Filter(vec![
                    ("key1".to_string(), "val1".to_string()),
                    ("key2".to_string(), "val2".to_string())
                ])
        );
    }

    #[test]
    fn test_parse_filter_segment_with_whitespaces() {
        let input = "[ key1 = val1, key2=val2]";
        let result = parse_filter_segment(input);
        check!(result.is_ok());
        let (rest, segment) = result.unwrap();
        check!(rest == "");
        check!(
            segment
                == Segment::Filter(vec![
                    ("key1".to_string(), "val1".to_string()),
                    ("key2".to_string(), "val2".to_string())
                ])
        );
    }

    #[test]
    fn test_parse_key_segment() {
        let input = "my_field";
        let result = parse_key_segment(input);
        check!(result.is_ok());
        let (rest, segment) = result.unwrap();
        check!(rest == "");
        check!(segment == Segment::Field(String::from("my_field")));
    }

    #[test]
    fn test_parse_condition() {
        let input = "name=JohnDoe";
        let result = parse_condition(input);
        check!(result.is_ok());
        let (rest, (key, value)) = result.unwrap();
        check!(rest == "");
        check!(key == "name");
        check!(value == "JohnDoe");
    }

    #[test]
    fn test_parse_value() {
        let input = "SomeValue123";
        let result = parse_value(input);
        check!(result.is_ok());
        let (rest, value) = result.unwrap();
        check!(rest == "");
        check!(value == "SomeValue123");
    }

    #[test]
    fn test_parse_ident() {
        let input = "field_name-1";
        let result = parse_ident(input);
        check!(result.is_ok());
        let (rest, ident) = result.unwrap();
        check!(rest == "");
        check!(ident == "field_name-1");
    }

    #[test]
    fn test_parse_path_with_whitespaces() {
        let input = "/ a / b / [ id = foo ] / c ";
        let result = parse_path(input);
        check!(result.is_ok());
        let (rest, spath) = result.unwrap();
        check!(rest == "");
        check!(spath.segments.len() == 4);
        check!(spath.segments[0] == Segment::Field(String::from("a")));
        check!(spath.segments[1] == Segment::Field(String::from("b")));
        check!(spath.segments[2] == Segment::Filter(vec![("id".to_string(), "foo".to_string())]));
        check!(spath.segments[3] == Segment::Field(String::from("c")));
    }

    #[test]
    fn test_parse_empty_path() {
        let input = "/";
        let result = parse_path(input);
        check!(result.is_ok());
        let (rest, spath) = result.unwrap();
        check!(rest == "");
        check!(spath.segments.is_empty());
    }

    #[test]
    fn test_parse_path_with_only_filters() {
        let input = "/[key=val]/[id=123]";
        let result = parse_path(input);
        check!(result.is_ok());
        let (rest, spath) = result.unwrap();
        check!(rest == "");
        check!(spath.segments.len() == 2);
        check!(spath.segments[0] == Segment::Filter(vec![("key".to_string(), "val".to_string())]));
        check!(spath.segments[1] == Segment::Filter(vec![("id".to_string(), "123".to_string())]));
    }

    #[test]
    fn test_parse_path_with_indexes() {
        let input = "/array/0/item";
        let result = parse_path(input);
        check!(result.is_ok());
        let (rest, spath) = result.unwrap();
        check!(rest == "");
        check!(spath.segments.len() == 3);
        check!(spath.segments[0] == Segment::Field(String::from("array")));
        check!(spath.segments[1] == Segment::Field(String::from("0")));
        check!(spath.segments[2] == Segment::Field(String::from("item")));
    }

    #[test]
    fn test_parse_invalid_path() {
        let input = "invalid_path";
        let result = parse_path(input);
        check!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_filter() {
        let input = "[keyval]";
        let result = parse_filter_segment(input);
        check!(result.is_err());
    }

    #[test]
    fn test_parse_filter_with_invalid_condition() {
        let input = "[key=val,invalidcondition]";
        let result = parse_filter_segment(input);
        check!(result.is_err());
    }

    #[test]
    fn test_parse_filter_with_empty_condition() {
        let input = "[]";
        let result = parse_filter_segment(input);
        check!(result.is_err());
    }
}
