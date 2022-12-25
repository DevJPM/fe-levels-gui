/*use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc
};

use fe_levels::{BlankAvoidance, Character, GrowthType, Stat, StatChange, StatType};
use itertools::Itertools;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(tag = "type"))]
enum JSONStatChange {
    LevelUp {
        temp_growth_bonus : HashMap<String, GrowthType>,
        retries_to_avoid_blank : u32
    },
    Promotion {
        promo_bonuses : HashMap<String, StatType>,
        new_caps : HashMap<String, StatType>
    }
}

impl Into<StatChange<String>> for JSONStatChange {
    fn into(self) -> StatChange<String> {
        match self {
            JSONStatChange::Promotion {
                promo_bonuses,
                new_caps
            } => StatChange::Promotion {
                promo_changes : Arc::new(move |name, current| Stat {
                    value : current.value + promo_bonuses.get(name).unwrap(),
                    growth : current.growth,
                    cap : *new_caps.get(name).unwrap()
                })
            },
            JSONStatChange::LevelUp {
                temp_growth_bonus,
                retries_to_avoid_blank
            } => StatChange::LevelUp {
                temporary_growth_override : Some(Arc::new(move |name, current| {
                    current.saturating_add(*temp_growth_bonus.get(name).unwrap_or(&0))
                })),
                blank_avoidance : BlankAvoidance::RetriesForNoBlank(retries_to_avoid_blank)
            }
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
struct JSONStat {
    base : StatType,
    growth : GrowthType,
    cap : StatType
}

impl Into<Stat> for JSONStat {
    fn into(self) -> Stat {
        Stat {
            cap : self.cap,
            growth : self.growth,
            value : self.base
        }
    }
}

#[derive(Serialize, Deserialize)]
struct JSONCharacter {
    stats : HashMap<String, JSONStat>
}

impl Into<Character<String>> for JSONCharacter {
    fn into(self) -> Character<String> {
        Character {
            stats : self.stats.into_iter().map(|(n, s)| (n, s.into())).collect()
        }
    }
}

#[derive(Serialize, Deserialize)]
struct JSONTask {
    character : JSONCharacter,
    stat_changes : Vec<JSONStatChange>
}

impl Into<(Character<String>, Vec<StatChange<String>>)> for JSONTask {
    fn into(self) -> (Character<String>, Vec<StatChange<String>>) {
        (
            self.character.into(),
            self.stat_changes.into_iter().map_into().collect()
        )
    }
}*/

fn main() {
/*    let task : JSONTask =
        serde_json::from_str(&std::fs::read_to_string("./character.json").unwrap()).unwrap();
    let (char, levels) = task.into();
    let result = fe_levels::generate_histograms(&levels, &char, 1e12 as u64);
    let result = result
        .into_iter()
        .map(|m| {
            m.into_iter()
                .map(|(s, h)| {
                    (
                        s,
                        h.iter_recorded()
                            .map(|iv| (iv.value_iterated_to(), iv.count_at_value()))
                            //.sorted_by_key(|(stat_value, _occurences)| *stat_value)
                            //.filter(|(_stat_value, occurences)| occurences > &0)
                            .collect::<BTreeMap<_, _>>()
                    )
                })
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let result = serde_json::to_string_pretty(&result).unwrap();
    std::fs::write("./data.json", result).unwrap();
    */
}
