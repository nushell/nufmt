use nu_parser::{lex, Token, TokenContents};

/// Checks if sequence of bytes has comments and split the code parts
pub fn split_by_comments(original_input: &[u8]) -> Vec<Token> {
    // if you use a signature, the last parameter should be true
    // nu_parser::lex_signature(input, span_offset, additional_whitespace, special_tokens, skip_comment)
    let lex_result: (Vec<Token>, Option<nu_protocol::ParseError>) =
        lex(original_input, 0, &[], &[], false);

    let tokens_vec = &lex_result.0;

    // copy the content (it will be splitted after)
    let mut current_content = original_input.clone();
    println!("length of content: {:?}", current_content.len());

    let mut comments_vec: Vec<&Token> = vec![];
    for (i, token) in tokens_vec.iter().enumerate() {
        println!("token: {i}, {:#?}", token.contents);
        if token.contents == TokenContents::Comment && next_token_is_eol(tokens_vec, i) {
            println!("Found a comment!");
            println!("Comment location: {:?}", token.span);

            // split twice
            // first, you split at the beginning of the comment
            let (beginning_to_comment, comment_to_end) = current_content.split_at(token.span.start);
            // println!("previous_to_comment {:?}", beginning_to_comment.len());
            // println!("comment_to_end {:?}", comment_to_end.len());
            // and then you split at the length of the comment
            let (comment, after_the_comment_to_end) =
                comment_to_end.split_at(token.span.end - token.span.start);
            // println!("comment {:?}", comment.len());
            // println!("after_the_comment {:?}", after_the_comment_to_end.len());

            // move the current content
            current_content = after_the_comment_to_end;

            (beginning_to_comment, comment)
        }
    }

    lex_result.0
}

/// Peeks at the next token and retuns true if the next token is Eol
fn next_token_is_eol(tokens_vec: &[Token], current_index: usize) -> bool {
    match tokens_vec.get(current_index + 1) {
        Some(next_token) => next_token.contents == TokenContents::Eol,
        None => false,
    }
}
