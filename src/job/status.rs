#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    UnknownState,
    Running,
    Exited(Option<i32>),
    Stopped(StopType),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StopType {
    Stop,
    Kill,
}

impl StopType {
    pub fn flag(&self) -> &str {
        match self {
            Self::Stop => "-STOP",
            Self::Kill => "-9",
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Stop => "stop",
            Self::Kill => "kill",
        }
    }
}
