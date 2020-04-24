use std::cell::Ref;

use cssparser::{ParseError, Parser, ParserInput, ToCss, Token, TokenSerializationType};
use html5ever::tendril::StrTendril;

pub fn write_to(
    mut previous_token: TokenSerializationType,
    input: &mut Parser,
    string: &mut String,
    preserve_comments: bool,
) {
    while let Ok(token) = if preserve_comments {
        input
            .next_including_whitespace_and_comments()
            .map(|t| t.clone())
    } else {
        input.next_including_whitespace().map(|t| t.clone())
    } {
        let token_type = token.serialization_type();
        if !preserve_comments && previous_token.needs_separator_when_before(token_type) {
            string.push_str("/**/")
        }
        previous_token = token_type;
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
                write_to(previous_token, input, string, preserve_comments);
                Ok(())
            });
            result.unwrap();
            closing_token.to_css(string).unwrap();
        }
    }
}

pub fn parse_and_serialize(input: Ref<StrTendril>, output: &mut String, preserve_comments: bool) {
    let mut parser_input = ParserInput::new(&input);
    let parser = &mut Parser::new(&mut parser_input);
    write_to(
        TokenSerializationType::nothing(),
        parser,
        output,
        preserve_comments,
    );
}
