#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    UnknownState,
    Running,
    Exited(Option<i32>),
    Stopped(StopType),
}

impl Status {
    pub fn is_stopped(&self) -> bool {
        match self {
            Self::Stopped(_) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StopType {
    Term,
    Kill,
}

impl StopType {
    pub fn sig(&self) -> usize {
        match self {
            Self::Term => 15,
            Self::Kill => 9,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Term => "term",
            Self::Kill => "kill",
        }
    }
}
