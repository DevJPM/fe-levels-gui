use std::{
    collections::BTreeMap,
    hash::Hash,
    ops::{Bound, RangeBounds},
    sync::Arc
};

use crate::analysis::binomial_analysis;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
pub type GrowthType = u16;
pub type StatType = u16;

pub const GUARANTEED_STAT_POINT_GROWTH : GrowthType = 100;

pub trait StatIndexType: Ord + Clone + Eq + Serialize + for<'a> Deserialize<'a> {}

impl<T : Ord + Clone + Eq + Serialize + for<'a> Deserialize<'a>> StatIndexType for T {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Stat {
    pub base : StatType,
    pub cap : StatType,
    pub growth : GrowthType,
    pub value : StatType
}

impl Stat {
    pub fn increase_value(&mut self, amount : StatType) {
        self.value = self.value.saturating_add(amount).clamp(self.base, self.cap)
    }
}

#[serde_as]
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Character<SIT : StatIndexType> {
    #[serde_as(as = "Vec<(_, _)>")]
    pub stats : BTreeMap<SIT, Stat>,
    pub name : String,
    pub level : usize
}

// TODO: handle FE11 dynamic growths
// TODO: handle FE12 drill ground growths which is sum of growths chance to
// determine the number of stats which can then still be rolled on capped stats
// unlike in FE10
// TODO: Handle FE12's ability to grow a stat and then only apply it after a
// promotion with a higher cap

pub struct DynamicGrowthData {
    pub num_prior_levels : u32 //?
}

pub enum BlankAvoidance<SIT : StatIndexType> {
    NoAvoidance,
    GuaranteedStats((Bound<u8>, Bound<u8>), Vec<SIT>), /* for FE10 and FE16, FE10 uses 3..=3
                                                       * here for
                                                       * BEXP,
                                                       * 1.. else and FE16 uses 2.. for
                                                       * students and
                                                       * byleth
                                                        Vec is iteration order
                                                       */
    /// This implements GBA FE Semantics
    /// GBA FE uses 2 re-rolls
    /// That is, a re-roll is only triggered if you didn't hit any growth
    /// If you hit a roll on a capped stat, the re-roll is not triggered
    RetriesForNoBlank(u32),
    /// This implements FE12 Drill Ground mechanics
    VariableGuaranteedStats,
    /// This implements FE15 (SoV) semantics
    /// SoV uses HP as the stat to award
    /// That is, it will award the named stat if you didn't hit any growth
    /// If you hit a roll on a capped stat, the award is not triggered
    /// If the named stat is already capped, nothing will be awarded on an empty
    /// level-up
    AwardFixedStatOnBlank(SIT)
}

impl<SIT : StatIndexType> BlankAvoidance<SIT> {
    pub fn new_guaranteed_stats(num_stats : impl RangeBounds<u8>) -> Self {
        BlankAvoidance::GuaranteedStats(
            (
                num_stats.start_bound().cloned(),
                num_stats.end_bound().cloned()
            ),
            vec![]
        )
    }
}

pub enum StatChange<SIT : StatIndexType> {
    LevelUp {
        temporary_growth_override : Option<Arc<dyn Fn(&SIT, GrowthType) -> GrowthType>>,
        blank_avoidance : BlankAvoidance<SIT>
    },
    Promotion {
        promo_changes : Arc<dyn Fn(&SIT, Stat) -> Stat>
    }
}

pub fn generate_histograms<SIT : StatIndexType>(
    levels : &[StatChange<SIT>],
    character : &Character<SIT>,
    num_samples : Option<u64>
) -> Vec<BTreeMap<SIT, BTreeMap<StatType, f64>>> {
    if let Some(analysis_result) = binomial_analysis(levels, character) {
        return analysis_result;
    }

    // TODO: First call into the analysis on the levels
    // then if the analysis rejects the level pattern
    // call into the simulation

    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
}
