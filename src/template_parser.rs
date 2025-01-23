use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::multispace0,
    combinator::map,
    error::Error as NomError,
    multi::many0,
    sequence::{delimited, preceded, terminated},
    IResult,
};

#[derive(Debug, PartialEq)]
pub enum TemplateSegment {
    Text(String),
    Variable(String),
    EachLoop {
        path: String,
        body: Vec<TemplateSegment>,
    },
}

impl ToString for TemplateSegment {
    fn to_string(&self) -> String {
        match self {
            TemplateSegment::Text(text) => text.clone(),
            TemplateSegment::Variable(var) => format!("{{{{{}}}}}", var),
            TemplateSegment::EachLoop { path, body } => {
                let body_str = body.iter().map(|s| s.to_string()).collect::<String>();
                format!("{{{{#each {}}}}}{}{{{{/each}}}}", path, body_str)
            }
        }
    }
}

fn parse_variable(input: &str) -> IResult<&str, TemplateSegment, NomError<&str>> {
    let var_parser = delimited(
        tag("{{"),
        preceded(
            multispace0,
            terminated(
                take_until::<_, _, NomError<&str>>("}}"),
                multispace0,
            ),
        ),
        tag("}}"),
    );
    
    map(var_parser, |var: &str| {
        TemplateSegment::Variable(var.trim().to_string())
    })(input)
}

fn parse_each_loop(input: &str) -> IResult<&str, TemplateSegment, NomError<&str>> {
    let (input, _) = tag("{{")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("#each")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, path) = take_until::<_, _, NomError<&str>>("}}")(input)?;
    let (input, _) = tag("}}")(input)?;
    
    // Capture the body including any newlines
    let (input, body) = take_until::<_, _, NomError<&str>>("{{/each}}")(input)?;
    let (input, _) = tag("{{/each}}")(input)?;
    
    let path = path.trim().to_string();
    
    // Parse the body, preserving all whitespace and newlines
    let (_, body_segments) = parse_template(body)?;
    
    // Preserve body segments exactly as parsed
    let mut final_segments = body_segments;
    
    Ok((input, TemplateSegment::EachLoop {
        path,
        body: final_segments,
    }))
}

fn parse_text(input: &str) -> IResult<&str, TemplateSegment, NomError<&str>> {
    // First try to parse until next template tag
    let result = take_until::<_, _, NomError<&str>>("{{");
    match result(input) {
        Ok((remaining, text)) => {
            Ok((remaining, TemplateSegment::Text(text.to_string())))
        },
        // If no template tags found, take all remaining text
        Err(_) => {
            if !input.is_empty() {
                Ok(("", TemplateSegment::Text(input.to_string())))
            } else {
                Err(nom::Err::Error(NomError::new(
                    input,
                    nom::error::ErrorKind::TakeUntil,
                )))
            }
        }
    }
}

pub fn parse_template(input: &str) -> IResult<&str, Vec<TemplateSegment>, NomError<&str>> {
    many0(alt((
        parse_each_loop,
        parse_variable,
        parse_text,
    )))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_variable() {
        let input = "{{user.name}}";
        let (rest, result) = parse_variable(input).unwrap();
        assert_eq!(rest, "");
        assert_eq!(result, TemplateSegment::Variable("user.name".to_string()));
    }

    #[test]
    fn test_parse_each_loop() {
        let input = "{{#each items}}{{.}}{{/each}}";
        let (rest, result) = parse_each_loop(input).unwrap();
        assert_eq!(rest, "");
        assert_eq!(
            result,
            TemplateSegment::EachLoop {
                path: "items".to_string(),
                body: vec![
                    TemplateSegment::Variable(".".to_string()),
                ],
            }
        );
    }

    #[test]
    fn test_parse_template() {
        let input = "Hello {{user.name}}! {{#each hobbies}}* {{.}}{{/each}}";
        let (rest, result) = parse_template(input).unwrap();
        assert_eq!(rest, "");
        assert_eq!(
            result,
            vec![
                TemplateSegment::Text("Hello ".to_string()),
                TemplateSegment::Variable("user.name".to_string()),
                TemplateSegment::Text("! ".to_string()),
                TemplateSegment::EachLoop {
                    path: "hobbies".to_string(),
                    body: vec![
                        TemplateSegment::Text("* ".to_string()),
                        TemplateSegment::Variable(".".to_string()),
                    ],
                },
            ]
        );
    }

    #[test]
    fn test_parse_template_with_newlines() {
        let input = "Hello\n{{#each items}}\n{{.}}\n{{/each}}\nGoodbye";
        let (rest, result) = parse_template(input).unwrap();
        assert_eq!(rest, "");
        assert_eq!(
            result,
            vec![
                TemplateSegment::Text("Hello\n".to_string()),
                TemplateSegment::EachLoop {
                    path: "items".to_string(),
                    body: vec![
                        TemplateSegment::Text("\n".to_string()),
                        TemplateSegment::Variable(".".to_string()),
                        TemplateSegment::Text("\n".to_string()),
                    ],
                },
                TemplateSegment::Text("\nGoodbye".to_string()),
            ]
        );
    }

    #[test]
    fn test_parse_text_with_newlines() {
        let input = "First line\nSecond line\nThird line";
        let (rest, result) = parse_text(input).unwrap();
        assert_eq!(rest, "");
        assert_eq!(result, TemplateSegment::Text("First line\nSecond line\nThird line".to_string()));
    }
}