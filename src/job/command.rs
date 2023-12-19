#[derive(Clone, Debug)]
pub struct Command {
    name: &'static str,
    args: Vec<String>,
}

impl Command {
    pub fn new(name: &'static str, args: Vec<String>) -> Self {
        Command { name, args }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn args(&self) -> Vec<String> {
        self.args.clone()
    }
}
