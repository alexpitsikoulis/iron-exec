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

    pub fn as_str(&self) -> &str {
        match self {
            Self::UnknownState => "Unknown state",
            Self::Running => "Running",
            Self::Exited(_) => "Exited",
            Self::Stopped(stop_type) => match stop_type {
                StopType::Term => "terminated",
                StopType::Kill => "Killed",
            },
        }
    }

    pub fn to_string(&self) -> String {
        String::from(self.as_str())
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
