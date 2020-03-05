use cssparser::{
    AtRuleParser, CowRcStr, DeclarationListParser, DeclarationParser, ParseError, Parser,
    ParserInput, QualifiedRuleParser, RuleListParser, SourceLocation,
};
use std::error::Error;

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
        let location = input.current_source_location();
        dbg!(&location);
        let position = input.position();
        dbg!(&position);
        while input.next().is_ok() {}
        Ok(input.slice_from(position).to_string())
    }

    fn parse_block<'t>(
        &mut self,
        selectors: Self::Prelude,
        _location: SourceLocation,
        input: &mut Parser<'i, 't>,
    ) -> Result<CssRule, CssParseError<'i>> {
        dbg!(&selectors);
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
        mut input: &mut Parser<'i, 't>,
    ) -> Result<Self::Declaration, ParseError<'i, CssError>> {
        Ok(vec![])
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
