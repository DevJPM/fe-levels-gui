use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    ops::RangeInclusive
};

use egui::{Button, ComboBox, Grid, Slider, TextEdit, Ui};
use fe_levels::StatType;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::app::{numerical_text_box, sit::StatIndexType, GameData, GameKind};

use super::UsableWeapon;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GbaWeaponClass {
    Dark,
    Anima,
    Light,
    Sword,
    Bow,
    Lance,
    Axe,
    Other
}

const ALL_WEAPON_CLASSES : [GbaWeaponClass; 8] = {
    use GbaWeaponClass::*;
    [Dark, Anima, Light, Sword, Bow, Lance, Axe, Other]
};

impl fmt::Display for GbaWeaponClass {
    fn fmt(&self, f : &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                GbaWeaponClass::Dark => "Dark",
                GbaWeaponClass::Anima => "Anima",
                GbaWeaponClass::Light => "Light",
                GbaWeaponClass::Sword => "Sword",
                GbaWeaponClass::Bow => "Bow",
                GbaWeaponClass::Lance => "Lance",
                GbaWeaponClass::Axe => "Axe",
                GbaWeaponClass::Other => "Other"
            }
        )
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum GbaSpecialProperties {
    Brave,
    Reaver,
    Heals,
    IgnoresDefense,
    MagicSword,
    Devil
}

const ALL_SPECIAL_PROPERTIES : [GbaSpecialProperties; 6] = {
    use GbaSpecialProperties::*;
    [Brave, Reaver, Heals, IgnoresDefense, MagicSword, Devil]
};

impl fmt::Display for GbaSpecialProperties {
    fn fmt(&self, f : &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                GbaSpecialProperties::Brave => "Brave",
                GbaSpecialProperties::Reaver => "Reaver",
                GbaSpecialProperties::Heals => "Heals",
                GbaSpecialProperties::IgnoresDefense => "Luna",
                GbaSpecialProperties::MagicSword => "Runesword",
                GbaSpecialProperties::Devil => "Devil"
            }
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GbaFeWeapon {
    weapon_class : GbaWeaponClass,
    might : StatType,
    weight : StatType,
    hitrate : StatType,
    critrate : StatType,
    name : String,
    range : RangeInclusive<u16>,
    stat_change : BTreeMap<StatIndexType, StatType>,
    special_properties : BTreeSet<GbaSpecialProperties>
}
impl UsableWeapon for GbaFeWeapon {
    fn name(&self) -> &str { &self.name }

    fn clarification_dialogue(mut self, context : &mut GameData, ui : &mut Ui) -> (Self, bool)
    where
        Self : Sized
    {
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.add(
                TextEdit::singleline(&mut self.name)
                    .desired_width(ui.spacing().text_edit_width * 0.88)
            );
            ComboBox::from_id_source("Special Weapon Properties")
                .selected_text("Special")
                .show_ui(ui, |ui| {
                    for property in ALL_SPECIAL_PROPERTIES {
                        let mut selected = self.special_properties.contains(&property);
                        ui.toggle_value(&mut selected, property.to_string());
                        if selected {
                            self.special_properties.insert(property);
                        }
                        else {
                            self.special_properties.remove(&property);
                        }
                    }
                });
        });

        Grid::new("GBA Weapon Grid")
            .max_col_width(ui.spacing().interact_size.x * 1.15)
            .show(ui, |ui| {
                ui.label("Class:");
                ComboBox::from_id_source("Weapon Class")
                    .selected_text(self.weapon_class.to_string())
                    .show_ui(ui, |ui| {
                        for class in ALL_WEAPON_CLASSES {
                            ui.selectable_value(&mut self.weapon_class, class, class.to_string());
                        }
                    });

                ui.label("Range:");
                ui.horizontal(|ui| {
                    let (mut start, mut end) = self.range.clone().into_inner();
                    numerical_text_box(ui, &mut start);
                    ui.label("-");
                    numerical_text_box(ui, &mut end);
                    self.range = RangeInclusive::new(start, end);
                });

                ui.label("Weight:");
                numerical_text_box(ui, &mut self.weight);
                ui.end_row();

                ui.label("Might:");
                numerical_text_box(ui, &mut self.might);

                ui.label("Hit:");
                numerical_text_box(ui, &mut self.hitrate);

                ui.label("Crit:");
                numerical_text_box(ui, &mut self.critrate);
                ui.end_row();
            });

        if self.stat_change.is_empty() {
            if ui.button("Add Stat Buff").clicked() {
                self.stat_change
                    .insert(StatIndexType::arbitrary_valid(GameKind::GbaFe), 0);
            }
        }
        else {
            Grid::new("Weapon Stat Buff Grid").show(ui, |ui| {
                let buffs = std::mem::take(&mut self.stat_change);
                let used_keys : BTreeSet<_> = buffs.keys().cloned().collect();
                let valid_keys : BTreeSet<_> = StatIndexType::new(GameKind::GbaFe)
                    .into_iter()
                    .filter(|sit| !used_keys.contains(sit))
                    .collect();
                for (mut index, mut buff) in buffs {
                    ComboBox::from_id_source(format!("{index} Combo-Box")).selected_text(index.to_string()).show_ui(ui, |ui| {
                        for index_option in valid_keys
                            .iter()
                            .map(|sit| *sit)
                            .chain(std::iter::once(index.clone()))
                            .sorted_by_key(|x| *x)
                        {
                            ui.selectable_value(&mut index, index_option, index_option.to_string());
                        }
                    });
                    ui.add(Slider::new(&mut buff, 0..=20).clamp_to_range(false));
                    let mut removed = false;
                    ui.horizontal(|ui| {
                        removed = ui.button("x").clicked();
                        if ui
                            .add_enabled(!valid_keys.is_empty(), Button::new("+"))
                            .clicked()
                        {
                            self.stat_change
                                .insert(valid_keys.first().unwrap().to_owned(), 0);
                        }
                    });

                    if !removed {
                        self.stat_change.insert(index, buff);
                    }
                    ui.end_row();
                }
            });
        }

        let confirmation_ready =
            context.weapons.check_legal_name(&self.name) && self.range.start() <= self.range.end();

        (
            self,
            ui.add_enabled(confirmation_ready, Button::new("confirm"))
                .on_disabled_hover_text(
                    "Please give this weapon a unique name and make sure the range is correct."
                )
                .clicked()
        )
    }
}

impl Default for GbaFeWeapon {
    fn default() -> Self {
        Self {
            weapon_class : GbaWeaponClass::Other,    // combo box
            might : 5,                               // slider, 0 - 25
            weight : 3,                              // slider, 0 - 25
            hitrate : 80,                            // slider, 60 - 200
            critrate : 10,                           // slider, 0 - 50
            name : Default::default(),               // textbox
            range : 1..=1,                           // double slider?
            special_properties : Default::default(), // combo box into x-able list
            stat_change : BTreeMap::new()            // x-able array of combo box + slider (0-20)
        }
    }
}
