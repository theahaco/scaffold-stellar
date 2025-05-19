use core::fmt::Display;

use loam_sdk::soroban_sdk::{self, contracttype, env, log, Env};

/// Represents the version of the contract
#[contracttype]
#[derive(Default, Eq, PartialEq, PartialOrd, Clone, Debug)]
pub struct Version(u32, u32, u32);

pub const INITAL_VERSION: Version = Version(0, 0, 1);

impl Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "v{}.{}.{}", self.major(), self.minor(), self.patch())
    }
}

impl Version {
    pub(crate) fn log(&self) {
        log!(env(), "v{}.{}.{}", self.major(), self.minor(), self.patch());
    }

    #[must_use]
    pub fn publish_patch(mut self) -> Self {
        self.2 += 1;
        self
    }

    #[must_use]
    pub fn publish_minor(mut self) -> Self {
        self.1 += 1;
        self.2 = 0;
        self
    }
    #[must_use]
    pub fn publish_major(mut self) -> Self {
        self.0 += 1;
        self.1 = 0;
        self.2 = 0;
        self
    }

    #[must_use]
    pub fn update(self, kind: &Update) -> Self {
        match kind {
            Update::Patch => self.publish_patch(),
            Update::Minor => self.publish_minor(),
            Update::Major => self.publish_major(),
        }
    }
    pub fn patch(&self) -> u32 {
        self.2
    }

    pub fn minor(&self) -> u32 {
        self.1
    }

    pub fn major(&self) -> u32 {
        self.0
    }
}

#[contracttype]
#[derive(Default)]
pub enum Update {
    #[default]
    Patch,
    Minor,
    Major,
}
