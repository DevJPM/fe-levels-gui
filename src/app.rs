use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    str::FromStr
};

use egui::{Button, Ui};
use fe_levels::{Character, StatType};
use itertools::Itertools;

use rand::random;
use serde::{Deserialize, Serialize};

use self::{
    manager::{DataManaged, check_legal_name},
    plotter::PlotterManager,
    progression::{ConcreteStatChange, ProgressionManager},
    sit::StatIndexType
};

mod manager;
mod plotter;
mod progression;
mod sit;

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
    game_option : GameKind,

    progression : ProgressionManager,

    promotions : DataManaged<Character<StatIndexType>>,
    characters : DataManaged<(Character<StatIndexType>, Vec<ConcreteStatChange>)>
}

impl Default for GameData {
    fn default() -> Self { generate_default_gamedata(Default::default()) }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Deserialize, Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct FeLevelGui {
    game_option : GameKind,

    game_data : HashMap<GameKind, GameData>
}

fn generate_default_gamedata(game_option : GameKind) -> GameData {
    GameData {
        plotter : Default::default(),
        character : StatIndexType::new_default_character(game_option),
        game_option,
        progression : Default::default(),
        promotions : Default::default(),
        characters : Default::default()
    }
}

fn numerical_text_box<T : Display + FromStr>(ui : &mut Ui, value : &mut T) {
    let mut text = value.to_string();
    ui.text_edit_singleline(&mut text);
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
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn character_builder(data : &mut GameData, ctx : &egui::Context) {
        egui::Window::new("Character Builder").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name: ");
                ui.text_edit_singleline(&mut data.character.name);
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

    fn data_manager(data : &mut GameData, ctx : &egui::Context) {
        data.characters.management_dialogue(
            ctx,
            "Character & Progression Manager",
            |(c, _p)| c.name.clone(),
            |ui, characters| {
                if check_legal_name(&data.character.name, characters) {
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

                ui.add_enabled_ui(characters.contains_key(&characters.selected), |ui| {
                    if ui.button("load character").clicked() {
                        data.character = characters.get(&characters.selected).unwrap().0.clone();
                    }
                    if ui.button("load progression").clicked() {
                        *data.progression = characters.get(&characters.selected).unwrap().1.clone();
                    }
                });
            }
        );
    }

    fn promotion_manager(data : &mut GameData, ctx : &egui::Context) {
        data.promotions
            .management_dialogue(ctx, "Promotion Manager", |c| c.name.clone(), |_, _| {})
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

        Self::character_builder(
            self.game_data
                .entry(self.game_option)
                .or_insert_with(|| generate_default_gamedata(self.game_option)),
            ctx
        );
        progression::character_progression_builder(
            self.game_data
                .entry(self.game_option)
                .or_insert_with(|| generate_default_gamedata(self.game_option)),
            ctx
        );
        plotter::data_plotting_windows(
            self.game_data
                .entry(self.game_option)
                .or_insert_with(|| generate_default_gamedata(self.game_option)),
            ctx
        );
        Self::data_manager(
            self.game_data
                .entry(self.game_option)
                .or_insert_with(|| generate_default_gamedata(self.game_option)),
            ctx
        );
        Self::promotion_manager(
            self.game_data
                .entry(self.game_option)
                .or_insert_with(|| generate_default_gamedata(self.game_option)),
            ctx
        );
    }
}
