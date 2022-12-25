use itertools::Itertools;
use repl_rs::Convert;
use std::{collections::{HashMap, BTreeMap}, fs, io, path::Path, sync::Arc};

use fe_levels::{BlankAvoidance, Character, GrowthType, Stat, StatChange, StatType};

use crate::{Arguments, Error, FeRepl, Return};

type GBASIT = String;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct GbaPromotion {
    growth_change : GrowthType,
    stat_bonus : HashMap<GBASIT, StatType>,
    new_caps : HashMap<GBASIT, StatType>
}

pub(crate) struct GbaFe {
    game : String,
    unit : Option<Character<GBASIT>>,
    progressions : Vec<(Option<String>, StatChange<GBASIT>)>,
    promotions : HashMap<String, GbaPromotion>
}

impl GbaFe {
    pub(crate) fn new(game : &str) -> Result<Self, Error> {
        let promotion_db = fs::OpenOptions::new()
            .read(true)
            .open(format!("./data/promotions/{game}.json"))?;

        Ok(GbaFe {
            game : game.to_string(),
            unit : None,
            progressions : vec![],
            promotions : serde_json::from_reader(promotion_db)?
        })
    }

    fn unit(&mut self) -> Result<&mut Character<String>, Error> {
        self.unit.as_mut().ok_or(Error::NoUnit)
    }

    fn name(&self) -> Result<&String, Error> { Ok(&self.unit.as_ref().ok_or(Error::NoUnit)?.name) }

    fn update_stat(
        &mut self,
        args : Arguments,
        extractor : impl Fn(&mut Stat) -> &mut StatType
    ) -> Result<(String, StatType, StatType), Error> {
        let input : String = args["stat"].convert()?;
        let (_score, stat) =
            find_closest(&input, &GBA_STATS).ok_or_else(|| Error::StatNotFound(input.clone()))?;
        let new_value : StatType = args["value"].convert()?;

        let val_ref = extractor(
            self.unit()?
                .stats
                .get_mut(stat)
                .ok_or(Error::StatNotFound(input))?
        );

        let old_value = *val_ref;
        *val_ref = new_value;

        Ok((stat.to_string(), old_value, new_value))
    }

    fn add_promotion_internal(&mut self, target_class : &str) -> Result<(), Error> {
        let promotion = self
            .promotions
            .get(target_class)
            .ok_or(Error::NoPromotionFound(target_class.to_string()))?
            .clone();
        self.progressions.push((
            Some(target_class.to_string()),
            StatChange::Promotion {
                promo_changes : Arc::new(move |name, mut stat| {
                    if !GBA_NON_GROWABLE_STATS.contains(&name.as_str()) {
                        stat.growth += promotion.growth_change;
                    }
                    if let Some(bonus) = promotion.stat_bonus.get(name) {
                        stat.base += bonus;
                        stat.value += bonus;
                    }
                    if let Some(new_cap) = promotion.new_caps.get(name) {
                        stat.cap = *new_cap;
                    }
                    stat
                })
            }
        ));
        Ok(())
    }
}

fn find_closest<'b>(input : &str, options : &[&'b str]) -> Option<(usize, &'b str)> {
    let best_matches = options
        .iter()
        .map(|vo| {
            (
                strsim::damerau_levenshtein(&input.to_lowercase(), &vo.to_lowercase()),
                vo
            )
        })
        .sorted_by_key(|(score, _value)| *score)
        .group_by(|(score, _value)| *score)
        .into_iter()
        .take(1)
        .flat_map(|(_score, group)| group)
        .collect_vec();

    if best_matches.len() != 1 {
        None
    }
    else {
        best_matches.first().map(|(score, value)| (*score, **value))
    }
}

const GBA_REFERENCE_BASE_STAT : Stat = Stat {
    base : 0,
    cap : 20,
    growth : 0,
    value : 0
};

const GBA_REFERENCE_LEVEL_UP : StatChange<GBASIT> = StatChange::LevelUp {
    temporary_growth_override : None,
    blank_avoidance : BlankAvoidance::RetriesForNoBlank(2)
};

const GBA_STATS : [&str; 9] = ["hp", "atk", "skl", "spd", "lck", "def", "res", "con", "mov"];
const GBA_NON_GROWABLE_STATS : [&str; 2] = ["con", "mov"];

impl FeRepl for GbaFe {
    fn new_unit(&mut self, args : Arguments) -> Return {
        let mut baseline_stats = BTreeMap::new();

        for stat in GBA_STATS {
            baseline_stats.insert(stat.to_string(), GBA_REFERENCE_BASE_STAT);
        }

        let name = args["name"].convert()?;

        let output_message = format!("Successfully created empty unit {name}.");

        self.unit = Some(Character {
            stats : baseline_stats,
            name
        });

        Ok(Some(output_message))
    }

    fn update_base(&mut self, args : Arguments) -> Return {
        let (stat, old, new) = self.update_stat(args, |s| &mut s.base)?;

        Ok(Some(format!(
            "Successfully updated {}'s {stat} base from {old} to {new}.",
            self.name()?
        )))
    }

    fn update_stat(&mut self, args : Arguments) -> Return {
        let (stat, old, new) = self.update_stat(args, |s| &mut s.value)?;

        Ok(Some(format!(
            "Successfully updated {}'s {stat} current stat value from {old} to {new}.",
            self.name()?
        )))
    }

    fn update_growth(&mut self, args : Arguments) -> Return {
        let (stat, old, new) = self.update_stat(args, |s| &mut s.growth)?;

        Ok(Some(format!(
            "Successfully updated {}'s {stat} growth from {old} to {new}.",
            self.name()?
        )))
    }

    fn update_cap(&mut self, args : Arguments) -> Return {
        let (stat, old, new) = self.update_stat(args, |s| &mut s.cap)?;

        Ok(Some(format!(
            "Successfully updated {}'s {stat} cap from {old} to {new}.",
            self.name()?
        )))
    }

    fn new_promotion(&mut self, args : Arguments) -> Return { todo!() }

    fn add_level(&mut self, _args : Arguments) -> Return {
        self.progressions.push((None, GBA_REFERENCE_LEVEL_UP));

        Ok(Some(format!(
            "Successfully added a new level-up to {}",
            self.name()?
        )))
    }

    fn add_promotion(&mut self, args : Arguments) -> Return {
        let target_class : String = args["target_class"].convert()?;

        self.add_promotion_internal(&target_class)?;

        Ok(Some(format!(
            "Successfully added a {target_class} promotion to {}'s progression.",
            self.name()?
        )))
    }

    fn heat_map(&mut self, args : Arguments) -> Return { todo!() }

    fn save_unit(&mut self, _args : Arguments) -> Return {
        let filename = format!(
            "./data/characters/{}/{}.json",
            self.game,
            self.name()?.to_lowercase()
        );

        let path = Path::new(&filename);

        std::fs::create_dir_all(&path.parent().ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{}", path.display())
        ))?)?;

        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)?;

        serde_json::to_writer_pretty(&file, &self.unit()?)?;

        Ok(Some(format!(
            "Successfully saved {} to {filename}",
            self.name()?
        )))
    }

    fn load_unit(&mut self, args : Arguments) -> Return {
        let loaded_unit : String = args["unit_name"].convert()?;

        let filename = format!(
            "./data/characters/{}/{}.json",
            self.game,
            loaded_unit.to_lowercase()
        );

        let file = std::fs::OpenOptions::new().read(true).open(&filename)?;

        let unit : Character<String> = serde_json::from_reader(&file)?;
        let name = unit.name.clone();

        self.unit = Some(unit);

        Ok(Some(format!("Successfully read {name} from {filename}")))
    }

    fn save_progression(&mut self, args : Arguments) -> Return {
        let filename : String = args["filename"].convert()?;
        let actual_filename = format!(
            "./data/progressions/{}/{}.json",
            self.game,
            filename.to_lowercase()
        );

        let path = Path::new(&actual_filename);

        std::fs::create_dir_all(&path.parent().ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{}", path.display())
        ))?)?;

        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)?;

        serde_json::to_writer_pretty(
            file,
            &self
                .progressions
                .iter()
                .map(|(indicator, _)| indicator)
                .collect_vec()
        )?;

        Ok(Some(format!(
            "Successfully saved the current progression for {} as \"{filename}\".",
            self.name()?
        )))
    }

    fn load_progression(&mut self, args : Arguments) -> Return {
        let filename : String = args["filename"].convert()?;
        let actual_filename = format!(
            "./data/progressions/{}/{}.json",
            self.game,
            filename.to_lowercase()
        );

        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(&actual_filename)?;

        let progression : Vec<Option<String>> = serde_json::from_reader(&file)?;

        self.progressions.clear();

        for item in progression {
            if let Some(promotion_name) = item {
                self.add_promotion_internal(&promotion_name)?;
            }
            else {
                self.progressions.push((None, GBA_REFERENCE_LEVEL_UP));
            }
        }

        Ok(Some(format!(
            "Successfully loaded the current progression for {} from \"{filename}\".",
            self.name()?
        )))
    }

    fn save_histograms(&mut self, args : Arguments) -> Return {
        // TODO: offer reduction to one stat type here and reduction to one specific
        // level-up
        // ... the latter one needs an index?
        // also maybe we should track a base-level for a character?
        todo!()
    }
}
