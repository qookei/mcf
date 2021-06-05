use std::fmt;

pub struct Fill {
	ch: char,
	len: usize
}

impl Fill {
	pub fn with(len: usize, ch: char) -> Fill {
		Fill {
			ch,
			len
		}
	}
}

impl fmt::Display for Fill {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for _ in 0..self.len {
			write!(f, "{}", self.ch)?;
		}
		Ok(())
	}
}
