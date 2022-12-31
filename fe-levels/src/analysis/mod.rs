use core::ops::Bound::Unbounded;
use std::{
    collections::BTreeMap,
    ops::{Bound, RangeBounds},
    sync::Arc
};

use contracts::debug_ensures;
use itertools::Itertools;

use crate::{
    BlankAvoidance, Character, GrowthType, Stat, StatChange, StatIndexType, StatType,
    GUARANTEED_STAT_POINT_GROWTH
};

const ERROR_BOUND : f64 = 1e-5;

fn validate_dist<SIT : StatIndexType>(stats : &BTreeMap<SIT, DistributedStat>) -> bool {
    stats.iter().all(|(_sit, ds)| validate_btree(&ds.stats))
}

fn validate_out<SIT : StatIndexType>(stats : &Vec<BTreeMap<SIT, BTreeMap<StatType, f64>>>) -> bool {
    stats
        .iter()
        .all(|stat| stat.iter().all(|(_sit, spread)| validate_btree(spread)))
}

fn validate_btree<K>(stats : &BTreeMap<K, f64>) -> bool {
    (stats.iter().map(|(_p, prob)| *prob).sum::<f64>() - 1.0).abs() < ERROR_BOUND
}

#[derive(Clone, Default)]
struct DistributedStat {
    growth : GrowthType,
    cap : StatType,
    stats : BTreeMap<StatType, f64>,
    base : StatType
}

#[debug_ensures(ret.as_ref().map(validate_out).unwrap_or(true))]
pub(crate) fn binomial_analysis<SIT>(
    levels : &[StatChange<SIT>],
    character : &Character<SIT>
) -> Option<Vec<BTreeMap<SIT, BTreeMap<StatType, f64>>>>
where
    SIT : StatIndexType
{
    if !levels.iter().all(binomial_stat_change_acceptable) {
        return None;
    }

    let mut collection : Vec<BTreeMap<SIT, DistributedStat>> = Vec::new();

    let current : BTreeMap<SIT, DistributedStat> = character
        .stats
        .iter()
        .map(|(sit, stat)| {
            let mut new_map = BTreeMap::new();
            new_map.insert(stat.value, 1.0);
            (
                sit.clone(),
                DistributedStat {
                    growth : stat.growth,
                    cap : stat.cap,
                    base : stat.base,
                    stats : new_map
                }
            )
        })
        .collect();
    collection.push(current.clone());

    collection.append(&mut levels.iter().scan(current, process_statchange).collect());

    Some(
        collection
            .into_iter()
            .map(|m| m.into_iter().map(|(i, sm)| (i, sm.stats)).collect())
            .collect()
    )
}

#[debug_ensures(ret.as_ref().map(|dist| validate_dist(dist)).unwrap_or(true))]
fn process_statchange<SIT : StatIndexType>(
    state : &mut BTreeMap<SIT, DistributedStat>,
    current_level : &StatChange<SIT>
) -> Option<BTreeMap<SIT, DistributedStat>> {
    match current_level {
        StatChange::LevelUp {
            temporary_growth_override,
            blank_avoidance,
            ..
        } => process_levelup(state, temporary_growth_override, blank_avoidance),
        StatChange::Promotion { promo_changes } => process_promotion(state, promo_changes)
    }
}

#[debug_ensures(ret.as_ref().map(|dist| validate_dist(dist)).unwrap_or(true))]
fn process_levelup<SIT : StatIndexType>(
    state : &mut BTreeMap<SIT, DistributedStat>,
    temporary_growth_override : &Option<Arc<dyn Fn(&SIT, u8) -> u8>>,
    blank_avoidance : &BlankAvoidance<SIT>
) -> Option<BTreeMap<SIT, DistributedStat>> {
    let old_ref = state.clone();

    let current_growths : BTreeMap<SIT, GrowthType> = old_ref
        .iter()
        .map(|(sit, ds)| {
            (
                sit.clone(),
                temporary_growth_override
                    .as_ref()
                    .map_or(ds.growth, |f| f(sit, ds.growth))
            )
        })
        .collect();

    let all_zero_prob : f64 = current_growths
        .iter()
        .map(|(sit, g)| (sit, (*g as f64) / (GUARANTEED_STAT_POINT_GROWTH as f64)))
        .map(|(_sit, g)| {
            if g >= 1.0 {
                0.0
            }
            else {
                1.0 - g
            }
        })
        .product();

    let guaranteed_growths = current_growths
        .iter()
        .map(|(sit, g)| (sit, g / GUARANTEED_STAT_POINT_GROWTH))
        .collect::<BTreeMap<_, _>>();
    let probabilistic_growths = current_growths
        .iter()
        .map(|(sit, g)| {
            (
                sit,
                ((g % GUARANTEED_STAT_POINT_GROWTH) as f64) / (GUARANTEED_STAT_POINT_GROWTH as f64)
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut updated_stats = BTreeMap::new();

    for data in old_ref.iter() {
        match blank_avoidance {
            BlankAvoidance::NoAvoidance => handle_simple_levelup(
                &guaranteed_growths,
                data,
                &probabilistic_growths,
                &mut updated_stats
            ),
            BlankAvoidance::RetriesForNoBlank(retries) => handle_retried_levelup(
                &guaranteed_growths,
                data,
                &probabilistic_growths,
                all_zero_prob,
                &mut updated_stats,
                *retries
            ),
            BlankAvoidance::AwardFixedStatOnBlank(backup_stat) => handle_fixed_stat_levelup(
                &guaranteed_growths,
                data,
                &probabilistic_growths,
                all_zero_prob,
                &mut updated_stats,
                backup_stat
            ),
            BlankAvoidance::GuaranteedStats(range, _order)
                if range.contains(&0) && range.end_bound() == Bound::Unbounded =>
            {
                handle_simple_levelup(
                    &guaranteed_growths,
                    data,
                    &probabilistic_growths,
                    &mut updated_stats
                )
            },
            /*BlankAvoidance::GuaranteedStats(range, order)
                if range.start_bound() == range.end_bound() =>
            {
                handle_guaranteed_stat_levelup(
                    &guaranteed_growths,
                    &old_ref,
                    &probabilistic_growths,
                    &mut updated_stats,
                    range,
                    order
                )
            },*/
            _ => panic!()
        }
    }

    *state = updated_stats;

    Some(state.clone())
}

fn handle_guaranteed_stat_levelup<SIT>(
    guaranteed_growths : &BTreeMap<&SIT, u8>,
    previous : &BTreeMap<SIT, DistributedStat>,
    probabilistic_growths : &BTreeMap<&SIT, f64>,
    updated_stats : &mut BTreeMap<SIT, DistributedStat>,
    range : &(Bound<u8>, Bound<u8>),
    order : &[SIT]
) where
    SIT : StatIndexType
{
    let mut iterator = order.iter().cycle().cloned();
    let mut awarded_stats = 0;

    for (key, ds) in previous.iter() {
        let guaranteed_growth = *guaranteed_growths.get(key).unwrap();
        if guaranteed_growth > 0 {
            awarded_stats += 1;
        }
        let mut acc = BTreeMap::new();
        for (stat_value, probability) in ds.stats.iter() {
            *acc.entry(
                stat_value
                    .saturating_add(guaranteed_growth)
                    .clamp(0, ds.cap)
            )
            .or_insert(0.0) += probability;
        }
        updated_stats.insert(
            key.clone(),
            DistributedStat {
                growth : ds.growth,
                cap : ds.cap,
                base : ds.base,
                stats : acc
            }
        );
    }

    // iterate the stats in order
    // then for each stat apply the growth probability (if it wouldn't violate a
    // cap) check whether we hit the guaranteed range (terminate if so) else
    // recurse with the next stat
    // and if we did not apply, recurse into the next stat
    // and at the start check how deep into the recursion we are and stop around
    // 20-30

    todo!()
}

/*

fn handle_guaranteed_stat_levelup_recursive<SIT>(
    probabilistic_growths : &HashMap<&SIT, f64>,
    updated_stats : &mut HashMap<SIT, DistributedStat>,
    range : &(Bound<u8>, Bound<u8>),
    iterator : impl Iterator<Item = SIT>,
    awarded_stats : u8,
    current_baseline_probability : f64,
    stats_probabilitistically_awarded : HashSet<SIT>,
    order : &[SIT],
    exponential_depth : u32,
    max_exponential_depth : u32
) where
    SIT : StatIndexType
{
    if range.contains(&awarded_stats) {
        return;
    }
    if current_baseline_probability <= 0.0 {
        return;
    }
    if exponential_depth >= max_exponential_depth {
        return;
    }
    if order
        .iter()
        .all(|sit| stats_probabilitistically_awarded.contains(sit))
    {
        return;
    }

    let current_stat = iterator.next().unwrap();

    if stats_probabilitistically_awarded.contains(&current_stat) {
        return handle_guaranteed_stat_levelup_recursive(
            probabilistic_growths,
            updated_stats,
            range,
            iterator,
            awarded_stats,
            current_baseline_probability,
            stats_probabilitistically_awarded,
            order,
            exponential_depth,
            max_exponential_depth
        );
    }

    // case 1: award the stat and not capped (add to set)
    // case 2: don't award the stat by probability
    // case 3: don't award the stat by cap (important for termination, add to
    // set, only recurse here if there's a non-zero chance of hitting the cap
    // before)
}

*/

fn handle_simple_levelup<SIT : StatIndexType>(
    guaranteed_growths : &BTreeMap<&SIT, u8>,
    (sit, ds) : (&SIT, &DistributedStat),
    probabilistic_growths : &BTreeMap<&SIT, f64>,
    updated_stats : &mut BTreeMap<SIT, DistributedStat>
) {
    let guaranteed_growth = *guaranteed_growths.get(sit).unwrap();
    let probabilistic_growth = *probabilistic_growths.get(sit).unwrap();
    let cap = ds.cap;
    let mut acc = BTreeMap::new();
    for (stat_value, probability) in ds.stats.iter() {
        *acc.entry(
            stat_value
                .saturating_add(guaranteed_growth + 1)
                .clamp(0, cap)
        )
        .or_insert(0.0) += probability * probabilistic_growth;
        *acc.entry(stat_value.saturating_add(guaranteed_growth).clamp(0, cap))
            .or_insert(0.0) += probability * (1.0 - probabilistic_growth);
    }
    updated_stats.insert(
        sit.clone(),
        DistributedStat {
            growth : ds.growth,
            cap,
            stats : acc,
            base : ds.base
        }
    );
}

fn handle_retried_levelup<SIT : StatIndexType>(
    guaranteed_growths : &BTreeMap<&SIT, u8>,
    (sit, ds) : (&SIT, &DistributedStat),
    probabilistic_growths : &BTreeMap<&SIT, f64>,
    all_zero_prob : f64,
    updated_stats : &mut BTreeMap<SIT, DistributedStat>,
    retries : u32
) {
    let guaranteed_growth = *guaranteed_growths.get(sit).unwrap();
    let probabilistic_growth = *probabilistic_growths.get(sit).unwrap();
    let cap = ds.cap;
    let all_others_zero = all_zero_prob / (1f64 - probabilistic_growth);
    let mut acc = BTreeMap::new();
    for iter in 0..=retries {
        let reroll_adjustment = if iter == retries {
            1.0
        }
        else {
            1.0 - all_others_zero
        };

        let scaling_factor = all_zero_prob.powi(iter as i32);

        for (stat_value, probability) in ds.stats.iter() {
            *acc.entry(
                stat_value
                    .saturating_add(guaranteed_growth + 1)
                    .clamp(0, cap)
            )
            .or_insert(0.0) += probability * probabilistic_growth * scaling_factor;
            *acc.entry(stat_value.saturating_add(guaranteed_growth).clamp(0, cap))
                .or_insert(0.0) +=
                probability * (1.0 - probabilistic_growth) * reroll_adjustment * scaling_factor;
        }
    }
    updated_stats.insert(
        sit.clone(),
        DistributedStat {
            growth : ds.growth,
            cap,
            stats : acc,
            base : ds.base
        }
    );
}

fn handle_fixed_stat_levelup<SIT : StatIndexType>(
    guaranteed_growths : &BTreeMap<&SIT, u8>,
    (sit, ds) : (&SIT, &DistributedStat),
    probabilistic_growths : &BTreeMap<&SIT, f64>,
    all_zero_prob : f64,
    updated_stats : &mut BTreeMap<SIT, DistributedStat>,
    backup_stat : &SIT
) {
    if backup_stat != sit {
        return handle_simple_levelup(
            guaranteed_growths,
            (sit, ds),
            probabilistic_growths,
            updated_stats
        );
    }

    let guaranteed_growth = *guaranteed_growths.get(sit).unwrap();
    let probabilistic_growth = *probabilistic_growths.get(sit).unwrap();
    let cap = ds.cap;
    let all_others_zero = all_zero_prob / (1f64 - probabilistic_growth);
    let mut acc = BTreeMap::new();

    for (stat_value, probability) in ds.stats.iter() {
        *acc.entry(
            stat_value
                .saturating_add(guaranteed_growth + 1)
                .clamp(0, cap)
        )
        .or_insert(0.0) += probability * (probabilistic_growth + all_zero_prob);
        *acc.entry(stat_value.saturating_add(guaranteed_growth).clamp(0, cap))
            .or_insert(0.0) +=
            probability * (1f64 - probabilistic_growth) * (1f64 - all_others_zero);
    }

    updated_stats.insert(
        sit.clone(),
        DistributedStat {
            growth : ds.growth,
            cap,
            stats : acc,
            base : ds.base
        }
    );
}

#[debug_ensures(ret.as_ref().map(|dist| validate_dist(dist)).unwrap_or(true))]
fn process_promotion<SIT : StatIndexType>(
    state : &mut BTreeMap<SIT, DistributedStat>,
    promo_changes : &Arc<dyn Fn(&SIT, Stat) -> Stat>
) -> Option<BTreeMap<SIT, DistributedStat>> {
    let old_ref = state.clone();

    let updated_state = old_ref
        .into_iter()
        .map(|(sit, ds)| internal_process_promotion(sit, ds, promo_changes))
        .collect::<BTreeMap<_, _>>();

    *state = updated_state;

    Some(state.clone())
}

#[debug_ensures(validate_btree(&ret.1.stats))]
fn internal_process_promotion<SIT : StatIndexType>(
    sit : SIT,
    ds : DistributedStat,
    promo_changes : &Arc<dyn Fn(&SIT, Stat) -> Stat>
) -> (SIT, DistributedStat) {
    let processed : Vec<_> = ds
        .stats
        .iter()
        .map(|(v, p)| {
            (
                promo_changes(
                    &sit,
                    Stat {
                        value : *v,
                        growth : ds.growth,
                        cap : ds.cap,
                        base : ds.base
                    }
                ),
                *p
            )
        })
        .collect();

    if !processed
        .iter()
        .map(|(s, _p)| (s.growth, s.cap))
        .all_equal()
    {
        panic!("found stat-dependent growths and caps! Crashing.");
    }

    let growth = processed.first().unwrap().0.growth;
    let cap = processed.first().unwrap().0.cap;

    (
        sit,
        DistributedStat {
            cap,
            growth,
            stats : processed
                .into_iter()
                .map(|(s, p)| (s.value, p))
                .sorted_by_key(|(k, _v)| *k)
                .group_by(|(k, _v)| *k)
                .into_iter()
                .map(|(points, group)| {
                    (points, group.into_iter().map(|(_points, prob)| prob).sum())
                })
                .collect(),
            base : ds.base
        }
    )
}

fn binomial_stat_change_acceptable<SIT : StatIndexType>(stat_change : &StatChange<SIT>) -> bool {
    match stat_change {
        StatChange::LevelUp {
            blank_avoidance: BlankAvoidance::GuaranteedStats(num_stats, _),
            ..
        } => {
            (num_stats.contains(&0) && num_stats.end_bound() == Unbounded)
                || num_stats.start_bound() == num_stats.end_bound()
        },
        StatChange::LevelUp {
            blank_avoidance: BlankAvoidance::VariableGuaranteedStats,
            ..
        } => false,
        _ => true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
