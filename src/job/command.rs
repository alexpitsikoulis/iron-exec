#[derive(Clone, Debug)]
pub struct Command {
    name: String,
    args: Vec<String>,
}

impl Command {
    pub fn new(name: String, args: Vec<String>) -> Self {
        Command { name, args }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn args(&self) -> Vec<String> {
        self.args.clone()
    }
}
