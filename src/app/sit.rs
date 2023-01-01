use std::fmt;

use fe_levels::{Character, Stat};
use serde::{Deserialize, Serialize};

use super::GameKind;

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Clone, Debug, Copy)]
pub struct StatIndexType(usize, GameKind);

impl PartialOrd for StatIndexType {
    fn partial_cmp(&self, other : &Self) -> Option<std::cmp::Ordering> {
        Some(Self::cmp(self, other))
    }
}

impl Ord for StatIndexType {
    fn cmp(&self, other : &Self) -> std::cmp::Ordering {
        //assert!(self.1 == other.1);
        usize::cmp(&self.0, &other.0)
    }
}

impl fmt::Display for StatIndexType {
    fn fmt(&self, f : &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(id, kind) = self;
        write!(
            f,
            "{}",
            look_up_iteration_order(*kind)
                .get(*id)
                .ok_or_else(|| fmt::Error::default())?
        )
    }
}

impl StatIndexType {
    pub fn new(game_option : GameKind) -> Vec<Self> {
        look_up_iteration_order(game_option)
            .into_iter()
            .enumerate()
            .map(|(i, _)| i)
            .map(|i| StatIndexType(i, game_option))
            .collect()
    }

    pub fn arbitrary_valid(game_option : GameKind) -> Self {
        *Self::new(game_option).first().unwrap()
    }

    pub fn is_hp(&self) -> bool { self.0 == 0 }

    pub fn is_luck(&self) -> bool {
        self.0
            == match self.1 {
                GameKind::GbaFe => 4,
                GameKind::PoR => 5
            }
    }

    pub fn default_stat(&self) -> Stat {
        let Self(_index, game) = self;
        match game {
            GameKind::GbaFe => {
                let cap = if self.is_hp() {
                    60
                }
                else if self.is_luck() {
                    30
                }
                else {
                    20
                };
                Stat {
                    base : cap / 4,
                    cap,
                    growth : 40,
                    value : cap / 4
                }
            },
            GameKind::PoR => {
                let cap = if self.is_hp() || self.is_luck() {
                    40
                }
                else {
                    20
                };
                Stat {
                    base : cap / 4,
                    cap,
                    growth : 40,
                    value : cap / 4
                }
            }
        }
    }

    pub fn new_default_character(game_option : GameKind) -> Character<Self> {
        Character {
            stats : Self::new(game_option)
                .into_iter()
                .map(|sit| (sit, sit.default_stat()))
                .collect(),
            name : "".to_string()
        }
    }
}

const TEMPLATE_INDEX : usize = 100;
pub const fn template_stat(game : GameKind) -> StatIndexType { StatIndexType(TEMPLATE_INDEX, game) }

const GBA_FE_ORDER : [&str; 7] = ["HP", "Atk", "Skl", "Spd", "Lck", "Def", "Res"];
const POR_ORDER : [&str; 8] = ["HP", "Str", "Mag", "SKl", "Spd", "Lck", "Def", "Res"];

fn look_up_iteration_order(game : GameKind) -> Vec<&'static str> {
    match game {
        GameKind::GbaFe => Vec::from(GBA_FE_ORDER),
        GameKind::PoR => Vec::from(POR_ORDER)
    }
}
