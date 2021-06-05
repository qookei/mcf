use std::fmt;
use std::iter::Peekable;
use std::str::CharIndices;

#[derive(Debug)]
pub enum TokenKind {
	LParen,
	RParen,
	LBracket,
	RBracket,
	Quote,
	Name(String),
	Integer(i64),
	String(String)
}

#[derive(Debug)]
pub struct Token {
	pub kind: TokenKind,
	pub pos: usize
}

impl Token {
	fn new_simple(ch: char, pos: usize) -> Token {
		Token {
			kind: match ch {
				'(' => TokenKind::LParen,
				')' => TokenKind::RParen,
				'[' => TokenKind::LBracket,
				']' => TokenKind::RBracket,
				'\'' => TokenKind::Quote,
				_ => unreachable!()
			},
			pos
		}
	}

	fn new_name(name: String, pos: usize) -> Token {
		Token {
			kind: TokenKind::Name(name),
			pos
		}
	}

	fn new_integer(value: i64, pos: usize) -> Token {
		Token {
			kind: TokenKind::Integer(value),
			pos
		}
	}

	fn new_string(value: String, pos: usize) -> Token {
		Token {
			kind: TokenKind::String(value),
			pos
		}
	}
}

impl fmt::Display for Token {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let readable_name = match self.kind {
			TokenKind::LParen => "opening parenthesis",
			TokenKind::RParen => "closing parenthesis",
			TokenKind::LBracket => "opening bracket",
			TokenKind::RBracket => "closing bracket",
			TokenKind::Quote => "quote",
			TokenKind::Name(_) => "name",
			TokenKind::Integer(_) => "integer",
			TokenKind::String(_) => "string"
		};

		write!(f, "{}", readable_name)
	}
}

#[derive(Debug)]
pub struct TokenizeError {
	pub message: String,
	pub pos: usize
}

struct Consumed {
	this: char,
	next: Option<char>,
	pos: usize
}

pub struct Tokenizer<'a> {
	it: Peekable<CharIndices<'a>>
}

impl<'a> Tokenizer<'a> {
	pub fn new_from_source(source: &'a str) -> Tokenizer {
		Tokenizer {
			it: source.char_indices().peekable(),
		}
	}

	fn consume_next(&mut self) -> Option<Consumed> {
		let (pos, this) = self.it.next()?;
		let next = self.it.peek().map(|v| v.1);

		Some(Consumed{this, next, pos})
	}

	pub fn tokenize(&mut self) -> Result<Vec<Token>, TokenizeError> {
		let mut tokens = Vec::<Token>::new();

		while let Some(c) = self.consume_next() {
			match (c.this, c.next) {
				('('|')'|'['|']'|'\'', _) => tokens.push(Token::new_simple(c.this, c.pos)),
				('"', _) => {
					let mut content = String::new();

					loop {
						let s = self.consume_next();
						if let Some(c) = s {
							let v = if c.this == '\\' {
								if let Some(next) = c.next {
									self.consume_next();

									match next {
										'"' => '"',
										't' => '\t',
										'n' => '\n',
										_ => return Err(TokenizeError{
											message: format!("Unknown escape sequence '\\{}'", next),
											pos: c.pos
										})
									}
								} else {
									return Err(TokenizeError{
										message: "Unexpected end of file".to_string(),
										pos: c.pos
									});
								}
							} else if c.this == '"' {
								break;
							} else {
								c.this
							};

							content.push(v);
						} else {
							return Err(TokenizeError{
								message: "Unterminated string".to_string(),
								pos: c.pos
							});
						}
					}

					tokens.push(Token::new_string(content, c.pos));
				},
				('0'..='9', _)|('-', Some('0'..='9')) => {
					let sign: i64 = if c.this == '-' { -1 } else { 1 };
					let mut value: i64 = if c.this == '-' { 0 } else { c.this.to_digit(10).unwrap() as i64 };

					let base = if c.this == '0' && c.next == Some('x') {
						self.consume_next();
						16
					} else {
						10
					};

					while let Some((_, ch)) = self.it.peek() {
						if ch.is_whitespace() || matches!(ch, ')'|']') {
							break;
						}

						let s = self.consume_next().unwrap();

						if !s.this.is_digit(base) {
							return Err(TokenizeError{
								message: "Unexpected character in integer literal".to_string(),
								pos: s.pos
							});
						}

						value *= base as i64;
						value += s.this.to_digit(base).unwrap() as i64;
					}

					value *= sign;

					tokens.push(Token::new_integer(value, c.pos));
				},
				('#', _) => {
					while let Some(c) = self.consume_next() {
						if c.this == '\n' {
							break;
						}
					}
				},
				_ if !c.this.is_whitespace() => {
					let mut name = String::new();

					name.push(c.this);

					while let Some((_, ch)) = self.it.peek() {
						if ch.is_whitespace() || matches!(ch, '('|')'|'"') {
							break;
						}

						let s = self.consume_next().unwrap();

						name.push(s.this);
					}

					tokens.push(Token::new_name(name, c.pos));
				},
				_ => {}
			}
		}

		Ok(tokens)
	}
}
