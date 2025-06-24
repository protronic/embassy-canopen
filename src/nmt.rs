
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum NmtState {
    Unknown,
    Initializing,
    Stopped,
    Operational,
    PreOperational,
}

impl From<u8> for NmtState {
    fn from(value: u8) -> Self {
        match value {
            0 => NmtState::Initializing,
            5 => NmtState::Operational,
            4 => NmtState::Stopped,
            127 => NmtState::PreOperational,
            _ => NmtState::Unknown
        }
    }
}

impl Into<u8> for NmtState {
    fn into(self) -> u8 {
        match self {
            NmtState::Initializing => 0,
            NmtState::Operational => 5,
            NmtState::Stopped => 4,
            NmtState::PreOperational => 127,
            NmtState::Unknown => 0xff
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum NmtCommand {
    Unknown,
    EnterOperational,
    EnterStopped,
    EnterPreOperational,
    ResetDevice,
    ResetCommunication,
}

impl From<u8> for NmtCommand {
    fn from(value: u8) -> Self {
        match value {
            1 => NmtCommand::EnterOperational,
            2 => NmtCommand::EnterStopped,
            128 => NmtCommand::EnterPreOperational,
            129 => NmtCommand::ResetDevice,
            130 => NmtCommand::ResetCommunication,
            _ => NmtCommand::Unknown
        }
    }
}

impl Into<u8> for NmtCommand {
    fn into(self) -> u8 {
        match self {
            NmtCommand::Unknown => 0xff,
            NmtCommand::EnterOperational => 1,
            NmtCommand::EnterStopped => 2,
            NmtCommand::EnterPreOperational => 128,
            NmtCommand::ResetDevice => 129,
            NmtCommand::ResetCommunication => 130
        }
    }
}