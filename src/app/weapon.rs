use egui::Ui;
use serde::{Deserialize, Serialize};

use self::gba::GbaFeWeapon;

use super::{GameData, GameKind};

mod gba;

#[derive(Serialize, Deserialize, Debug)]
pub enum Weapon {
    GbaFeWeapon(GbaFeWeapon),
    PoRWeapon
}

impl Weapon {
    pub fn new(game_option : GameKind) -> Self {
        match game_option {
            GameKind::GbaFe => Self::GbaFeWeapon(GbaFeWeapon::default()),
            GameKind::PoR => Self::PoRWeapon
        }
    }
}

pub trait UsableWeapon {
    fn name(&self) -> &str;

    fn clarification_dialogue(self, context : &mut GameData, ui : &mut Ui) -> (Self, bool)
    where
        Self : Sized;
}

impl UsableWeapon for Weapon {
    fn name(&self) -> &str {
        match self {
            Weapon::GbaFeWeapon(data) => data.name(),
            Weapon::PoRWeapon => ""
        }
    }

    fn clarification_dialogue(self, context : &mut GameData, ui : &mut Ui) -> (Self, bool)
    where
        Self : Sized
    {
        match self {
            Weapon::GbaFeWeapon(data) => {
                let (weapon, ready) = data.clarification_dialogue(context, ui);
                (Self::GbaFeWeapon(weapon), ready)
            },
            Weapon::PoRWeapon => (self, true)
        }
    }
}
