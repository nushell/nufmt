use nu_parser::{lex, Token, TokenContents};

/// Checks if sequence of bytes has comments
pub fn split_by_comments(contents: &[u8]) -> Vec<Token> {
    let lex_result: (Vec<Token>, Option<nu_protocol::ParseError>) =
        lex(contents, 0, &[], &[], false);
    // if you use a signature, the last parameter should be true
    // nu_parser::lex_signature(input, span_offset, additional_whitespace, special_tokens, skip_comment)

    let token_vec = &lex_result.0;
    let mut previous_token = TokenContents::Eol;
    for (i, token) in token_vec.iter().enumerate() {
        println!("token: {i}, {:#?}", token.contents);
        if token.contents == TokenContents::Eol && previous_token == TokenContents::Comment {
            println!("Found a comment!")
        }
        // put the curren token into the previous token var
        previous_token = token.contents
    }

    lex_result.0
}
