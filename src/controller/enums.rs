#[repr(u8)]
#[derive(Debug)]
pub enum BeltState {
    Undefined = 2,
    Static = 0,
    Moving = 1,
}

impl From<u8> for BeltState {
    fn from(i: u8) -> Self {
        match i {
            0 => BeltState::Static,
            1 => BeltState::Moving,
            _ => BeltState::Undefined,
        }
    }
}

#[derive(Debug)]
pub enum Message {
    State(super::State),
}

#[repr(u8)]
#[derive(Debug)]
pub enum Mode {
    Undefined = 3,
    Standby = 2,
    Manual = 1,
    Automat = 0,
}

impl From<u8> for Mode {
    fn from(i: u8) -> Self {
        match i {
            0 => Mode::Automat,
            1 => Mode::Manual,
            2 => Mode::Standby,
            _ => Mode::Undefined,
        }
    }
}
