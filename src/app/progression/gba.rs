use std::{fmt, sync::Arc};

use egui::{Button, Grid, ScrollArea, Ui};
use fe_levels::{BlankAvoidance, Character, StatChange};
use serde::{Deserialize, Serialize};

use crate::app::{
    numerical_text_box,
    sit::{template_stat, StatIndexType},
    GameData, GameKind
};

use super::{PromotionSelectionKind, UsefulStatChange};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum GbaFeStatChange {
    Promotion(Character<StatIndexType>),
    LevelUp,
    GrowthBooster,
    StatBooster(StatIndexType)
}

impl UsefulStatChange for GbaFeStatChange {
    fn compile(self) -> StatChange<StatIndexType> {
        match self {
            GbaFeStatChange::Promotion(promotion_gains) => StatChange::Promotion {
                promo_changes : Arc::new(move |sit, mut stat| {
                    promotion_gains
                        .stats
                        .get(sit)
                        .map(|bonus| {
                            stat.cap = bonus.cap;
                            stat.increase_value(bonus.value);
                            stat
                        })
                        .unwrap_or(stat)
                })
            },
            GbaFeStatChange::LevelUp => StatChange::LevelUp {
                temporary_growth_override : None,
                blank_avoidance : BlankAvoidance::RetriesForNoBlank(2)
            },
            GbaFeStatChange::GrowthBooster => StatChange::Promotion {
                promo_changes : Arc::new(|_sit, mut stat| {
                    stat.growth = stat.growth.saturating_add(5);
                    stat
                })
            },
            GbaFeStatChange::StatBooster(boosted_sit) => StatChange::Promotion {
                promo_changes : Arc::new(move |sit, mut stat| {
                    if *sit == boosted_sit {
                        stat.increase_value(if boosted_sit.is_hp() { 7 } else { 2 })
                    }
                    stat
                })
            }
        }
    }

    fn marking_worthy(&self) -> bool {
        match self {
            GbaFeStatChange::Promotion(_) => true,
            GbaFeStatChange::LevelUp => false,
            GbaFeStatChange::GrowthBooster => false,
            GbaFeStatChange::StatBooster(_) => false
        }
    }

    fn increases_level_counter(&self) -> bool {
        match self {
            GbaFeStatChange::Promotion(_) => false,
            GbaFeStatChange::LevelUp => true,
            GbaFeStatChange::GrowthBooster => false,
            GbaFeStatChange::StatBooster(_) => false
        }
    }

    fn resets_level_counter(&self) -> bool {
        match self {
            GbaFeStatChange::Promotion(_) => true,
            GbaFeStatChange::LevelUp => false,
            GbaFeStatChange::GrowthBooster => false,
            GbaFeStatChange::StatBooster(_) => false
        }
    }

    fn clarification_dialogue(
        self,
        context : &mut GameData,
        ui : &mut Ui
    ) -> (GbaFeStatChange, bool) {
        match self {
            GbaFeStatChange::Promotion(mut promotion_gains) => {
                ui.horizontal(|ui| {
                    ui.radio_value(
                        &mut context.progression.promotion_selection_strategy,
                        PromotionSelectionKind::ManualPromotionEntry,
                        "Manual Promotion Entry"
                    );
                    ui.radio_value(
                        &mut context.progression.promotion_selection_strategy,
                        PromotionSelectionKind::LoadSavedPromotion,
                        "Select Saved Promotion"
                    );
                });

                match context.progression.promotion_selection_strategy {
                    PromotionSelectionKind::LoadSavedPromotion => {
                        ScrollArea::vertical().show_rows(
                            ui,
                            ui.text_style_height(&egui::TextStyle::Body),
                            context.progression.progression.len(),
                            |ui, range| {
                                for (name, promo) in
                                    context.promotions.iter().take(range.end).skip(range.start)
                                {
                                    ui.selectable_value(&mut promotion_gains, promo.clone(), name);
                                    ui.end_row();
                                }
                            }
                        );
                        let clicked = ui
                            .add_enabled(
                                context.promotions.contains_key(&promotion_gains.name),
                                Button::new("load")
                            )
                            .on_disabled_hover_text("Please select a promotion.")
                            .clicked();
                        (GbaFeStatChange::Promotion(promotion_gains), clicked)
                    },
                    PromotionSelectionKind::ManualPromotionEntry => {
                        ui.label("Promotion Target Class: ");
                        ui.text_edit_singleline(&mut promotion_gains.name);
                        Grid::new("Promotion Grid").num_columns(3).show(ui, |ui| {
                            ui.label("");
                            ui.label("promotion gain");
                            ui.label("new cap");
                            ui.end_row();

                            for (sit, stat) in promotion_gains.stats.iter_mut() {
                                ui.label(format!("{sit}"));
                                numerical_text_box(ui, &mut stat.value);
                                numerical_text_box(ui, &mut stat.cap);
                                ui.end_row();
                            }
                        });
                        let mut confirmed = false;
                        ui.horizontal(|ui| {
                            let name = &promotion_gains.name;
                            confirmed = ui
                                .add_enabled(!name.is_empty(), Button::new("confirm"))
                                .on_disabled_hover_text(
                                    "Please name the class you're promoting into."
                                )
                                .clicked();

                            if ui
                                .add_enabled(
                                    context.promotions.check_legal_name(&promotion_gains.name),
                                    Button::new("save")
                                )
                                .on_disabled_hover_text(
                                    "Please name the class you're promoting into and make sure \
                                     that you didn't previously save an equally named promotion."
                                )
                                .clicked()
                            {
                                context
                                    .promotions
                                    .insert(promotion_gains.name.clone(), promotion_gains.clone());
                            }
                        });

                        (GbaFeStatChange::Promotion(promotion_gains), confirmed)
                    }
                }
            },
            GbaFeStatChange::LevelUp => (self, true),
            GbaFeStatChange::GrowthBooster => (self, true),
            GbaFeStatChange::StatBooster(mut stat) => {
                if stat == template_stat(GameKind::GbaFe) {
                    stat = StatIndexType::new(GameKind::GbaFe)[0];
                }
                egui::containers::ComboBox::from_label("Stat to Boost")
                    .selected_text(format!("{}", stat))
                    .show_ui(ui, |ui| {
                        StatIndexType::new(GameKind::GbaFe).iter().for_each(|key| {
                            ui.selectable_value(&mut stat, *key, key.to_string());
                        });
                    });
                (
                    GbaFeStatChange::StatBooster(stat),
                    ui.button("Confirm").clicked()
                )
            }
        }
    }

    fn requires_clarification(&self) -> bool {
        match self {
            GbaFeStatChange::Promotion(_) => true,
            GbaFeStatChange::LevelUp => false,
            GbaFeStatChange::GrowthBooster => false,
            GbaFeStatChange::StatBooster(_) => true
        }
    }

    fn cheap_to_execute(&self) -> bool { true }

    fn generate_templates(game_option : GameKind) -> Vec<Self>
    where
        Self : Sized
    {
        debug_assert!(game_option == GameKind::GbaFe);
        vec![
            GbaFeStatChange::GrowthBooster,
            GbaFeStatChange::LevelUp,
            GbaFeStatChange::StatBooster(template_stat(GameKind::GbaFe)),
            GbaFeStatChange::Promotion(Character {
                stats : StatIndexType::new_default_character(GameKind::GbaFe)
                    .stats
                    .into_iter()
                    .map(|(sit, mut stat)| {
                        stat.growth = 0;
                        stat.value = 2;
                        if !sit.is_hp() && !sit.is_luck() {
                            stat.cap += 5;
                        };
                        (sit, stat)
                    })
                    .collect(),
                name : "".to_owned(),
                level : 1
            }),
        ]
    }
}

impl fmt::Display for GbaFeStatChange {
    fn fmt(&self, f : &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GbaFeStatChange::Promotion(promotion) => {
                if promotion.name.is_empty() {
                    write!(f, "Promotion")
                }
                else {
                    write!(f, "{} Promotion", promotion.name)
                }
            },
            GbaFeStatChange::LevelUp => write!(f, "Level-Up"),
            GbaFeStatChange::GrowthBooster => write!(f, "5% Growth-Booster"),
            GbaFeStatChange::StatBooster(stat) => {
                if stat == &template_stat(GameKind::GbaFe) {
                    write!(f, "Stat Booster")
                }
                else if stat.is_hp() {
                    write!(f, "+7 HP Booster") // this is the angelic robe
                }
                else {
                    write!(f, "+2 {stat} Booster")
                }
            }
        }
    }
}
