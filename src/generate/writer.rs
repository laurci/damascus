pub struct CodeWriter {
    output: String,
    indent_level: usize,
    indent_string: String,
}

impl CodeWriter {
    pub fn new() -> Self {
        Self::with_indent("  ")
    }

    pub fn with_indent(indent: &str) -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
            indent_string: indent.to_string(),
        }
    }

    pub fn into_string(self) -> String {
        self.output
    }

    pub fn line(&mut self, s: &str) {
        self.output
            .push_str(&self.indent_string.repeat(self.indent_level));
        self.output.push_str(s);
        self.output.push('\n');
    }

    pub fn empty_line(&mut self) {
        self.output.push('\n');
    }

    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    pub fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }

    pub fn block<F>(&mut self, opening: &str, closing: &str, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.line(opening);
        self.indent();
        f(self);
        self.dedent();
        self.line(closing);
    }

    pub fn block_with_newline<F>(&mut self, opening: &str, closing: &str, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.line(opening);
        self.indent();
        f(self);
        self.dedent();
        self.line(closing);
        self.empty_line();
    }
}
