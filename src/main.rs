use std::fs;
use std::process;

use std::iter::Peekable;
use std::slice::Iter;

mod lex;
mod util;

struct Pos<'a> {
	line: usize,
	column: usize,

	line_content: &'a str
}

impl<'a> Pos<'a> {
	fn from_offset(source: &'a str, pos: usize) -> Pos<'a> {
		let mut line: usize = 1;
		let mut column: usize = 1;

		for (idx, ch) in source.char_indices() {
			if idx == pos {
				break;
			}

			column += 1;
			if ch == '\n' {
				column = 1;
				line += 1;
			}
		}

		Pos { line, column, line_content: source.lines().nth(line - 1).unwrap() }
	}
}

#[derive(Debug)]
enum Expr {
	VariableRef{var: String},
	IntegerLiteral(i64),
	StringLiteral(String),
	FnCall{name: String, args: Vec<Expr>},
	Args{args: Vec<Expr>},
	DefineFn{name: String, args: Box<Expr>, body: Box<Expr>},
	Do{exprs: Vec<Expr>},
	Let{name: String, r#type: String},
}

#[derive(Debug)]
struct ParseError<'a> {
	message: String,
	token: &'a lex::Token
}

trait Error {
	fn position<'a>(&self, source: &'a str) -> Pos<'a>;
	fn message(&self) -> &String;
}

impl<'a> Error for ParseError<'a> {
	fn position<'b>(&self, source: &'b str) -> Pos<'b> {
		Pos::from_offset(source, self.token.pos)
	}

	fn message(&self) -> &String {
		&self.message
	}
}

impl Error for lex::TokenizeError {
	fn position<'a>(&self, source: &'a str) -> Pos<'a> {
		Pos::from_offset(source, self.pos)
	}

	fn message(&self) -> &String {
		&self.message
	}
}

fn report_error<T: Error>(source: &str, error: &T) -> ! {
	let pos = error.position(source);
	println!("Error at {}:{}: {}", pos.line, pos.column, error.message());
	println!(" {} | {}", pos.line, pos.line_content);
	println!(" {} | {}~", pos.line, util::Fill::with(pos.column - 1, ' '));
	process::exit(1);
}

struct Parser<'a> {
	it: Peekable<Iter<'a, lex::Token>>
}

impl<'a> Parser<'a> {
	fn new_from_tokens(tokens: &'a [lex::Token]) -> Parser<'a> {
		Parser {
			it: tokens.iter().peekable()
		}
	}

	fn parse_fncall(&mut self, name: &str) -> Result<Option<Expr>, ParseError<'a>> {
		let mut args = Vec::<Expr>::new();

		while let Some(tok) = self.it.peek() {
			if matches!(tok.kind, lex::TokenKind::RParen) {
				break;
			}

			args.push(self.parse_expr()?.unwrap());
		}

		Ok(Some(Expr::FnCall{name: name.to_string(), args}))
	}

	fn parse_do(&mut self) -> Result<Option<Expr>, ParseError<'a>> {
		let mut exprs = Vec::<Expr>::new();

		while let Some(tok) = self.it.peek() {
			if matches!(tok.kind, lex::TokenKind::RParen) {
				break;
			}

			exprs.push(self.parse_expr()?.unwrap());
		}

		Ok(Some(Expr::Do{exprs}))
	}

	fn parse_args(&mut self) -> Result<Option<Expr>, ParseError<'a>> {
		let mut args = Vec::<Expr>::new();

		while let Some(tok) = self.it.peek() {
			if matches!(tok.kind, lex::TokenKind::RParen) {
				break;
			}

			args.push(self.parse_expr()?.unwrap());
		}

		Ok(Some(Expr::Args{args}))
	}

	fn parse_definefn(&mut self, fn_token: &'a lex::Token) -> Result<Option<Expr>, ParseError<'a>> {
		let name_tok = self.it.next();

		let name = match name_tok {
			None => Err(ParseError{
				message: "Unexpected end of input, was a name for this function".to_string(),
				token: fn_token
			}),
			Some(lex::Token{kind: lex::TokenKind::Name(n), pos: _}) => Ok(n),
			/* TODO: Anonymous functions: */
			/* Some(lex::Token{kind: lex::TokenKind::LParen, pos: _}) => ..., */
			_ => Err(ParseError{
				message: "Unexpected token, was expecting a name".to_string(),
				token: name_tok.unwrap()
			})
		}?;

		let args = Box::new(self.parse_expr()?.unwrap());
		let body = Box::new(self.parse_expr()?.unwrap());

		Ok(Some(Expr::DefineFn{name: name.to_string(), args, body}))
	}

	fn parse_let(&mut self, let_token: &'a lex::Token) -> Result<Option<Expr>, ParseError<'a>> {
		let name_tok = self.it.next();

		let name = match name_tok {
			None => Err(ParseError{
				message: "Unexpected end of input, was a name for this variable".to_string(),
				token: let_token
			}),
			Some(lex::Token{kind: lex::TokenKind::Name(n), pos: _}) => Ok(n),
			_ => Err(ParseError{
				message: "Unexpected token, was expecting a name".to_string(),
				token: name_tok.unwrap()
			})
		}?;


		let type_tok = self.it.next();

		let r#type = match type_tok {
			None => Err(ParseError{
				message: "Unexpected end of input, was a type name for this variable".to_string(),
				token: let_token
			}),
			Some(lex::Token{kind: lex::TokenKind::Name(n), pos: _}) => Ok(n),
			_ => Err(ParseError{
				message: "Unexpected token, was expecting a type name".to_string(),
				token: type_tok.unwrap()
			})
		}?;

		Ok(Some(Expr::Let{name: name.to_string(), r#type: r#type.to_string()}))
	}

	fn parse_expr(&mut self) -> Result<Option<Expr>, ParseError<'a>> {
		if let Some(token) = self.it.next() {
			match &token.kind {
				lex::TokenKind::LParen => {
					if let Some(next) = self.it.next() {
						let name = match &next.kind {
							lex::TokenKind::Name(n) => Ok(n),
							_ => Err(ParseError{
								message: "Unexpected token, was expecting a name".to_string(),
								token: next
							})
						}?;

						let result = match name.as_str() {
							"fn" => self.parse_definefn(next),
							"let" => self.parse_let(next),
							"do" => self.parse_do(),
							"args" => self.parse_args(),
							_ => self.parse_fncall(name)
						}?;

						let rparen_tok = self.it.next();

						match rparen_tok {
							None => Err(ParseError{
								message: "Unexpected end of input, was expecting a closing parenthesis to close this expression".to_string(),
								token
							}),
							Some(lex::Token{kind: lex::TokenKind::RParen, pos: _}) => {
								Ok(result)
							},
							_ => {
								Err(ParseError{
									message: "Unexpected token, was expecting a closing parenthesis".to_string(),
									token: rparen_tok.unwrap()
								})
							}
						}
					} else {
						Err(ParseError{
							message: "Unexpected end of file, was expecting a name".to_string(),
							token
						})
					}
				},

				lex::TokenKind::Name(name) => {
					Ok(Some(Expr::VariableRef{var: name.to_string()}))
				},

				lex::TokenKind::Integer(val) => {
					Ok(Some(Expr::IntegerLiteral(*val)))
				},

				lex::TokenKind::String(val) => {
					Ok(Some(Expr::StringLiteral(val.to_string())))
				},

				_ => {
					Err(ParseError{
						message: format!("Unexpeced {}", token),
						token
					})
				}
			}
		} else {
			Ok(None)
		}
	}
}

fn main() {
	let contents = fs::read_to_string("test").unwrap();

	let mut tokenizer = lex::Tokenizer::new_from_source(&contents);
	let tokens = tokenizer.tokenize().unwrap_or_else(|e| report_error(&contents, &e));

	println!("Tokens: {:#?}", tokens);

	let mut parser = Parser::new_from_tokens(&tokens);

	loop {
		let expr = parser.parse_expr().unwrap_or_else(|e| report_error(&contents, &e));

		match expr {
			Some(e) => println!("Expr: {:#?}", e),
			None => { break; }
		}
	}
}
