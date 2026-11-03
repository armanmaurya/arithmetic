pub(super) struct Cursor<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub(super) fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    pub(super) fn pos(&self) -> usize {
        self.pos
    }

    pub(super) fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    pub(super) fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    pub(super) fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    pub(super) fn eat_while(&mut self, pred: impl Fn(char) -> bool) {
        while self.peek().is_some_and(&pred) {
            self.bump();
        }
    }

    pub(super) fn eat_str(&mut self, s: &str) -> bool {
        if self.rest().starts_with(s) {
            self.pos += s.len();
            true
        } else {
            false
        }
    }

    pub(super) fn slice_from(&self, start: usize) -> &'a str {
        &self.src[start..self.pos]
    }
}
