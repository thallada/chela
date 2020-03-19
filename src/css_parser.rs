use cssparser::{
    AtRuleParser, CowRcStr, DeclarationListParser, DeclarationParser, ParseError, Parser,
    ParserInput, QualifiedRuleParser, RuleListParser, SourceLocation,
};
use std::fmt;
use std::convert::Into;
use std::error::Error;

// TODO: try to use CowRcStr instead of Strings in these structs.
// I tried to do it earlier but I ended up in lifetime hell in parse_block().
#[derive(Debug)]
pub struct CssRule {
    pub selectors: String,
    pub declarations: Vec<CssDeclaration>,
}

#[derive(Debug)]
pub struct CssDeclaration {
    pub property: String,
    pub value: String,
}

#[derive(Debug)]
pub enum CssError {}

pub type CssParseError<'i> = ParseError<'i, CssError>;

struct CssParser;

impl<'i> AtRuleParser<'i> for CssParser {
    type PreludeBlock = ();
    type PreludeNoBlock = ();
    type AtRule = CssRule;
    type Error = CssError;
}

impl<'i> QualifiedRuleParser<'i> for CssParser {
    type Prelude = String;
    type QualifiedRule = CssRule;
    type Error = CssError;

    fn parse_prelude<'t>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, CssParseError<'i>> {
        let position = input.position();
        while input.next().is_ok() {}
        Ok(input.slice_from(position).to_string())
    }

    fn parse_block<'t>(
        &mut self,
        selectors: Self::Prelude,
        _location: SourceLocation,
        input: &mut Parser<'i, 't>,
    ) -> Result<CssRule, CssParseError<'i>> {
        Ok(CssRule {
            selectors,
            declarations: parse_declarations(input).unwrap(),
        })
    }
}

pub fn parse_css_stylesheet(css: &str) -> Vec<CssRule> {
    let mut parser_input = ParserInput::new(css);
    let mut parser = Parser::new(&mut parser_input);

    let rule_list_parser = RuleListParser::new_for_stylesheet(&mut parser, CssParser);

    let mut rules = Vec::new();

    for result in rule_list_parser {
        let rule = match result {
            Ok(r) => r,
            Err((error, string)) => {
                eprintln!("Rule dropped: {:?}, {:?}", error, string);
                continue;
            }
        };
        rules.push(rule);
    }

    rules
}

pub fn parse_css_style_attribute(css: &str) -> Vec<CssDeclaration> {
    let mut parser_input = ParserInput::new(css);
    let mut parser = Parser::new(&mut parser_input);

    parse_declarations(&mut parser).unwrap()
}

#[derive(Debug)]
struct CssDeclarationParser;

impl<'i> DeclarationParser<'i> for CssDeclarationParser {
    type Declaration = Vec<CssDeclaration>;
    type Error = CssError;

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Declaration, ParseError<'i, CssError>> {
        dbg!(&name);
        let start = input.position();
        input.next()?;
        let value = input.slice_from(start);
        dbg!(&value);

        Ok(vec![CssDeclaration {
            property: name.to_string(),
            value: value.trim().to_string(),
        }])
    }
}

impl<'i> AtRuleParser<'i> for CssDeclarationParser {
    type PreludeBlock = ();
    type PreludeNoBlock = ();
    type AtRule = Vec<CssDeclaration>;
    type Error = CssError;
}

pub fn parse_declarations<'i>(
    input: &mut Parser<'i, '_>,
) -> Result<Vec<CssDeclaration>, Box<dyn Error>> {
    let mut declarations = Vec::new();
    let declaration_list_parser = DeclarationListParser::new(input, CssDeclarationParser);

    for declaration_list in declaration_list_parser {
        let declaration_list = match declaration_list {
            Ok(l) => l,
            Err(e) => {
                eprintln!("CSS declaration dropped: {:?}", e);
                continue;
            }
        };
        for declaration in declaration_list {
            declarations.push(declaration);
        }
    }

    Ok(declarations)
}

impl fmt::Display for CssDeclaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {};", self.property, self.value)
    }
}

impl Into<String> for CssDeclaration {
    fn into(self) -> String {
        format!("{}: {};", self.property, self.value)
    }
}
