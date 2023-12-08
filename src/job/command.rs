#[derive(Clone, Copy, Debug)]
pub struct Command {
    name: &'static str,
    args: &'static [&'static str],
}

impl Command {
    pub fn new(name: &'static str, args: &'static [&'static str]) -> Self {
        Command { name, args }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn args(&self) -> &'static [&'static str] {
        self.args
    }
}