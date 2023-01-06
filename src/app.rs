use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    str::FromStr
};

use egui::{Button, TextEdit, Ui};
use fe_levels::{Character, StatType};
use itertools::Itertools;

use rand::random;
use serde::{Deserialize, Serialize};

use self::{
    manager::DataManaged,
    plotter::PlotterManager,
    progression::{ConcreteStatChange, ProgressionManager},
    sit::StatIndexType,
    weapon::{UsableWeapon, Weapon}
};

mod manager;
mod plotter;
mod progression;
mod sit;
mod weapon;

type CompleteData = Vec<BTreeMap<StatIndexType, BTreeMap<StatType, f64>>>;

#[derive(PartialEq, Default, Deserialize, Serialize, Hash, Eq, Clone, Copy, Debug)]
pub enum GameKind {
    #[default]
    GbaFe,
    PoR
}

#[derive(Deserialize, Serialize, Hash, PartialEq, Eq, Clone, Copy)]
struct UsefulId(u64);

impl Default for UsefulId {
    fn default() -> Self { Self(random()) }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum StatChangeTemplate {
    LevelUp
}

#[derive(Deserialize, Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct GameData {
    plotter : PlotterManager,

    character : Character<StatIndexType>,
    enemy : Option<Character<StatIndexType>>,
    weapon : Option<Weapon>,
    game_option : GameKind,

    progression : ProgressionManager,

    promotions : DataManaged<Character<StatIndexType>>,
    characters : DataManaged<(Character<StatIndexType>, Vec<ConcreteStatChange>)>,
    enemies : DataManaged<Character<StatIndexType>>,
    weapons : DataManaged<Weapon>
}

impl Default for GameData {
    fn default() -> Self { generate_default_gamedata(Default::default()) }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Deserialize, Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct FeLevelGui {
    version : u64,

    game_option : GameKind,

    game_data : HashMap<GameKind, GameData>
}

impl Default for FeLevelGui {
    fn default() -> Self {
        Self {
            version : 2,
            game_option : Default::default(),
            game_data : Default::default()
        }
    }
}

fn generate_default_gamedata(game_option : GameKind) -> GameData {
    GameData {
        plotter : Default::default(),
        character : StatIndexType::new_default_character(game_option),
        game_option,
        progression : Default::default(),
        promotions : Default::default(),
        characters : Default::default(),
        enemy : Default::default(),
        enemies : Default::default(),
        weapons : Default::default(),
        weapon : Default::default()
    }
}

fn numerical_text_box<T : Display + FromStr>(ui : &mut Ui, value : &mut T) {
    let mut text = value.to_string();
    ui.add(TextEdit::singleline(&mut text).desired_width(ui.spacing().text_edit_width));
    if let Ok(parsed) = str::parse(&text) {
        *value = parsed;
    }
    // do not write the result back / do anything in case of a bad parse
}

impl FeLevelGui {
    /// Called once before the first frame.
    pub fn new(cc : &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            let state : Self = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            if state.version < Self::default().version {
                return Default::default();
            }
            else {
                return state;
            }
        }

        Default::default()
    }

    fn character_builder(data : &mut GameData, ctx : &egui::Context) {
        egui::Window::new("Character Builder").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name: ");
                ui.add(
                    TextEdit::singleline(&mut data.character.name)
                        .desired_width(ui.spacing().slider_width * 1.5)
                );
                ui.label("Level: ");
                numerical_text_box(ui, &mut data.character.level);
            });
            egui::Grid::new("Character Builder Table").show(ui, |ui| {
                ui.label("Stat");
                ui.label("Base");
                ui.label("Cap");
                ui.label("Growth");
                ui.end_row();

                data.character
                    .stats
                    .iter_mut()
                    .sorted_by_key(|(key, _value)| **key)
                    .for_each(|(key, stat)| {
                        ui.label(key.to_string());
                        ui.add(egui::Slider::new(&mut stat.base, 0..=stat.cap));
                        stat.value = stat.base;
                        numerical_text_box(ui, &mut stat.cap);
                        numerical_text_box(ui, &mut stat.growth);
                        ui.end_row()
                    });
            });
        });
    }

    fn character_manager(data : &mut GameData, ctx : &egui::Context) {
        data.characters.management_dialogue(
            ctx,
            false,
            "Character & Progression Manager",
            |(c, _p)| c.name.clone(),
            |ui, characters| {
                if characters.check_legal_name(&data.character.name) {
                    if ui.button("save character & progression").clicked() {
                        characters.insert(
                            data.character.name.clone(),
                            (data.character.clone(), data.progression.clone())
                        );
                    }
                }
                else if ui
                    .add_enabled(
                        !data.character.name.is_empty(),
                        Button::new("overwrite character & progression")
                    )
                    .clicked()
                {
                    characters.insert(
                        data.character.name.clone(),
                        (data.character.clone(), data.progression.clone())
                    );
                }

                ui.add_enabled_ui(characters.selected().is_some(), |ui| {
                    if ui.button("load character").clicked() {
                        data.character = characters.selected().unwrap().0.clone();
                    }
                    if ui.button("load progression").clicked() {
                        *data.progression = characters.selected().unwrap().1.clone();
                    }
                });
            }
        );
    }

    fn enemy_manager(data : &mut GameData, ctx : &egui::Context) {
        let modal_rect = data.enemies.management_dialogue(
            ctx,
            data.enemy.is_some(),
            "Enemy Manager",
            |c| c.name.clone(),
            |ui, enemies| {
                if ui.button("add").clicked() {
                    data.enemy = Some(StatIndexType::new_default_enemy(data.game_option));
                }

                ui.add_enabled_ui(enemies.selected().is_some(), |ui| {
                    if ui.button("edit").clicked() {
                        let selected_name = enemies.selected().unwrap().name.clone();
                        data.enemy = enemies.remove(&selected_name);
                    }
                });
            }
        );

        if let Some(mut enemy) = std::mem::take(&mut data.enemy) {
            egui::Window::new("Enemy Builder")
                .fixed_rect(modal_rect.unwrap())
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name: ");
                        ui.text_edit_singleline(&mut enemy.name);
                    });
                    egui::Grid::new("Enemy Builder Table").show(ui, |ui| {
                        ui.label("Stat");
                        ui.label("Value");
                        ui.end_row();

                        enemy
                            .stats
                            .iter_mut()
                            .sorted_by_key(|(key, _value)| **key)
                            .for_each(|(key, stat)| {
                                ui.label(key.to_string());
                                numerical_text_box(ui, &mut stat.value);
                                ui.end_row()
                            });
                    });
                    if ui
                        .add_enabled(
                            data.enemies.check_legal_name(&enemy.name),
                            Button::new("confirm")
                        )
                        .clicked()
                    {
                        data.enemies.insert(enemy.name.clone(), enemy);
                    }
                    else {
                        data.enemy = Some(enemy)
                    }
                });
        }
    }

    fn promotion_manager(data : &mut GameData, ctx : &egui::Context) {
        data.promotions.management_dialogue(
            ctx,
            false,
            "Promotion Manager",
            |c| c.name.clone(),
            |_, _| {}
        );
    }

    fn weapon_manager(data : &mut GameData, ctx : &egui::Context) {
        let modal_rect = data.weapons.management_dialogue(
            ctx,
            data.weapon.is_some(),
            "Weapon Manager",
            |w| w.name().to_owned(),
            |ui, weapons| {
                if ui.button("add").clicked() {
                    data.weapon = Some(Weapon::new(data.game_option));
                }

                ui.add_enabled_ui(weapons.selected().is_some(), |ui| {
                    if ui.button("edit").clicked() {
                        let selected_name = weapons.selected().unwrap().name().to_owned();
                        data.weapon = weapons.remove(&selected_name);
                    }
                });
            }
        );

        if let Some(weapon) = std::mem::take(&mut data.weapon) {
            egui::Window::new("Weapon Builder")
                .fixed_rect(modal_rect.unwrap())
                .collapsible(false)
                .show(ctx, |ui| {
                    let (weapon, ready) = weapon.clarification_dialogue(data, ui);
                    if ready {
                        data.weapons.insert(weapon.name().to_owned(), weapon);
                    }
                    else {
                        data.weapon = Some(weapon);
                    }
                });
        }
    }
}

impl eframe::App for FeLevelGui {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage : &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per
    /// second. Put your widgets into a `SidePanel`, `TopPanel`,
    /// `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx : &egui::Context, _frame : &mut eframe::Frame) {
        egui::TopBottomPanel::top("Game Selector").show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::global_dark_light_mode_switch(ui);
                ui.label("Game Mechanics: ");
                ui.selectable_value(&mut self.game_option, GameKind::GbaFe, "GBA-FE");
                ui.selectable_value(&mut self.game_option, GameKind::PoR, "FE9");
            });
        });

        egui::CentralPanel::default().show(ctx, |_| {});

        let game_data = self
            .game_data
            .entry(self.game_option)
            .or_insert_with(|| generate_default_gamedata(self.game_option));

        Self::character_builder(game_data, ctx);
        progression::character_progression_builder(game_data, ctx);
        plotter::data_plotting_windows(game_data, ctx);
        Self::character_manager(game_data, ctx);
        Self::promotion_manager(game_data, ctx);
        Self::enemy_manager(game_data, ctx);
        Self::weapon_manager(game_data, ctx);
    }
}
