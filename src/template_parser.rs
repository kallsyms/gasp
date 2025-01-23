use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::multispace0,
    combinator::map,
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

fn parse_variable(input: &str) -> IResult<&str, TemplateSegment> {
    let var_parser = delimited(
        tag("{{"),
        preceded(
            multispace0,
            terminated(
                take_until("}}"),
                multispace0,
            ),
        ),
        tag("}}"),
    );
    
    map(var_parser, |var: &str| {
        TemplateSegment::Variable(var.trim().to_string())
    })(input)
}

fn parse_each_loop(input: &str) -> IResult<&str, TemplateSegment> {
    let (input, _) = tag("{{")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("#each")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, path) = take_until("}}")(input)?;
    let (input, _) = tag("}}")(input)?;
    let (input, body) = take_until("{{/each}}")(input)?;
    let (input, _) = tag("{{/each}}")(input)?;
    
    let path = path.trim().to_string();
    let (_, mut body_segments) = parse_template(body)?;
    
    Ok((input, TemplateSegment::EachLoop {
        path,
        body: body_segments,
    }))
}

fn parse_text(input: &str) -> IResult<&str, TemplateSegment> {
    let (input, text) = take_until("{{")(input)?;
    Ok((input, TemplateSegment::Text(text.to_string())))
}

pub fn parse_template(input: &str) -> IResult<&str, Vec<TemplateSegment>> {
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
}