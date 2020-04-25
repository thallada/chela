// Note: not using this parser. It would be easier to preserve whitespace in the css strings using
// this parser but it requires me to do too much of what cssparser is already doing for me
// (distinguishing between an at-rule `Ident` vs. a style rule `Ident`).
use std::borrow::Borrow;
use std::cell::Ref;

use cssparser::{ParseError, Parser, ParserInput, ToCss, Token, TokenSerializationType};
use html5ever::tendril::StrTendril;

use crate::css_property::CssProperty;
use crate::sanitizer::SanitizerConfig;

pub fn write_to(
    mut previous_token: TokenSerializationType,
    input: &mut Parser,
    string: &mut String,
    config: &SanitizerConfig,
    skipping_property: bool,
    skipping_at_rule: bool,
) {
    while let Ok(token) = if config.allow_css_comments {
        input
            .next_including_whitespace_and_comments()
            .map(|t| t.clone())
    } else {
        input.next_including_whitespace().map(|t| t.clone())
    } {
        let token_type = token.serialization_type();
        let mut skipping_property = skipping_property;
        let mut skipping_at_rule = skipping_at_rule;
        if !config.allow_css_comments && previous_token.needs_separator_when_before(token_type) {
            string.push_str("/**/")
        }
        previous_token = token_type;
        match &token {
            Token::Ident(property) => {
                let property_str: &str = property.borrow();
                if !config
                    .allowed_css_properties
                    .contains(&CssProperty::from(property_str))
                {
                    skipping_property = true;
                }
            }
            _ => {}
        }
        dbg!(&token);
        token.to_css(string).unwrap();
        let closing_token = match token {
            Token::Function(_) | Token::ParenthesisBlock => Some(Token::CloseParenthesis),
            Token::SquareBracketBlock => Some(Token::CloseSquareBracket),
            Token::CurlyBracketBlock => Some(Token::CloseCurlyBracket),
            _ => None,
        };
        if let Some(closing_token) = closing_token {
            let result: Result<_, ParseError<()>> = input.parse_nested_block(|input| {
                write_to(
                    previous_token,
                    input,
                    string,
                    config,
                    skipping_property,
                    skipping_at_rule,
                );
                Ok(())
            });
            result.unwrap();
            closing_token.to_css(string).unwrap();
        }
    }
}

pub fn parse_and_serialize(input: Ref<StrTendril>, output: &mut String, config: &SanitizerConfig) {
    let mut parser_input = ParserInput::new(&input);
    let parser = &mut Parser::new(&mut parser_input);
    write_to(
        TokenSerializationType::nothing(),
        parser,
        output,
        config,
        false,
        false,
    );
}
