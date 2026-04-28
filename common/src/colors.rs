use std::fmt;
use atty;

#[derive(Debug, Clone, Copy)]
pub struct Colors {
    pub enabled: bool,
}

impl Colors {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn auto() -> Self {
        Self::new(atty::is(atty::Stream::Stdout))
    }

    pub fn reset(&self) -> &'static str {
        if self.enabled { "\x1b[0m" } else { "" }
    }

    pub fn bold(&self) -> &'static str {
        if self.enabled { "\x1b[1m" } else { "" }
    }

    pub fn dim(&self) -> &'static str {
        if self.enabled { "\x1b[2m" } else { "" }
    }

    pub fn red(&self) -> &'static str {
        if self.enabled { "\x1b[31m" } else { "" }
    }

    pub fn green(&self) -> &'static str {
        if self.enabled { "\x1b[32m" } else { "" }
    }

    pub fn yellow(&self) -> &'static str {
        if self.enabled { "\x1b[33m" } else { "" }
    }

    pub fn blue(&self) -> &'static str {
        if self.enabled { "\x1b[34m" } else { "" }
    }

    pub fn magenta(&self) -> &'static str {
        if self.enabled { "\x1b[35m" } else { "" }
    }

    pub fn cyan(&self) -> &'static str {
        if self.enabled { "\x1b[36m" } else { "" }
    }

    pub fn white(&self) -> &'static str {
        if self.enabled { "\x1b[37m" } else { "" }
    }

    pub fn gray(&self) -> &'static str {
        if self.enabled { "\x1b[90m" } else { "" }
    }

    pub fn bright_red(&self) -> &'static str {
        if self.enabled { "\x1b[91m" } else { "" }
    }

    pub fn bright_green(&self) -> &'static str {
        if self.enabled { "\x1b[92m" } else { "" }
    }

    pub fn bright_yellow(&self) -> &'static str {
        if self.enabled { "\x1b[93m" } else { "" }
    }

    pub fn bright_blue(&self) -> &'static str {
        if self.enabled { "\x1b[94m" } else { "" }
    }
}

impl fmt::Display for Colors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Colors {{ enabled: {} }}", self.enabled)
    }
}
