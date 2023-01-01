use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    str::FromStr
};

use egui::{Button, ScrollArea, TextEdit, Ui};
use fe_levels::{Character, StatType};
use itertools::Itertools;

use rand::random;
use serde::{Deserialize, Serialize};

use self::{
    plotter::PlotterManager,
    progression::{ConcreteStatChange, ProgressionManager},
    sit::StatIndexType
};

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

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, Default)]
enum CodeEditMode {
    #[default]
    Export,
    Importing(String)
}

#[derive(Deserialize, Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct GameData {
    plotter : PlotterManager,

    character : Character<StatIndexType>,
    game_option : GameKind,

    progression : ProgressionManager,

    promotions : BTreeMap<String, Character<StatIndexType>>,
    characters : BTreeMap<String, (Character<StatIndexType>, Vec<ConcreteStatChange>)>,

    selected_promotion : String,
    renamed_promotion : Option<Character<StatIndexType>>,
    promotion_code_edit_mode : CodeEditMode,

    selected_character : String,
    renamed_character : Option<(Character<StatIndexType>, Vec<ConcreteStatChange>)>,
    character_code_edit_mode : CodeEditMode
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
        characters : Default::default(),
        selected_promotion : Default::default(),
        renamed_promotion : Default::default(),
        promotion_code_edit_mode : Default::default(),
        selected_character : Default::default(),
        renamed_character : Default::default(),
        character_code_edit_mode : Default::default()
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
        let inner_rect = egui::Window::new("Character & Progression Manager")
            .collapsible(data.renamed_character.is_none())
            .show(ctx, |ui| {
                ui.set_enabled(data.renamed_character.is_none());
                ui.columns(3, |uis| {
                    let ui = &mut uis[1];

                    if check_legal_name(&data.character.name, &data.characters) {
                        if ui.button("save character & progression").clicked() {
                            data.characters.insert(data.character.name.clone(), (data.character.clone(), data.progression.clone()));
                        }
                    }
                    else if ui.add_enabled(!data.character.name.is_empty(), Button::new("overwrite character & progression")).clicked() {
                        data.characters.insert(data.character.name.clone(), (data.character.clone(), data.progression.clone()));
                    }

                    ui.add_enabled_ui(
                        data.characters.contains_key(&data.selected_character),
                        |ui| {
                            if ui.button("load character").clicked() {
                                data.character = data.characters.get(&data.selected_character).unwrap().0.clone();
                            }
                            if ui.button("load progression").clicked() {
                                *data.progression = data.characters.get(&data.selected_character).unwrap().1.clone();
                            }
                            if ui.button("delete").clicked() {
                                data.characters.remove(&data.selected_character);
                            }
                            if ui.button("rename").clicked() {
                                data.renamed_character =
                                    data.characters.remove(&data.selected_character);
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                if ui.button("copy to clipboard").clicked() {
                                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                        let _best_effort = clipboard.set_text(
                                            serde_json::to_string(
                                                &data.characters.get(&data.selected_character)
                                            )
                                            .unwrap()
                                        );
                                    }
                                }
                            }
                        }
                    );

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let mut clipboard_copied_character : Option<(Character<StatIndexType>,Vec<ConcreteStatChange>)> = None;
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            if let Ok(text) = clipboard.get_text() {
                                if let Ok(parse) =
                                    serde_json::from_str::<(Character<StatIndexType>,Vec<ConcreteStatChange>)>(&text)
                                {
                                    if !check_legal_name(&parse.0.name, &data.characters) {
                                        clipboard_copied_character = Some(parse);
                                    }
                                }
                            }
                            ui.add_enabled_ui(clipboard_copied_character.is_some(), |ui| {
                                if ui.button("import from clipboard").clicked() {
                                    if let Some(clipboard_copied_promotion) =
                                        clipboard_copied_character
                                    {
                                        data.characters.insert(
                                            clipboard_copied_promotion.0.name.clone(),
                                            clipboard_copied_promotion
                                        );
                                    }
                                }
                            });
                        }
                    }

                    if ui.add_enabled(
                        data.character_code_edit_mode != CodeEditMode::Export,
                        Button::new("export json")
                    ).clicked() {
                        data.character_code_edit_mode = CodeEditMode::Export;
                    }
                    if ui.add_enabled(
                        matches!(data.character_code_edit_mode, CodeEditMode::Export)
                            || matches!(&data.character_code_edit_mode, CodeEditMode::Importing(s)
                             if check_legal_name(&serde_json::from_str::<(Character<StatIndexType>,Vec<ConcreteStatChange>)>(s).map(|(char, _progression)| char.name)
                             .unwrap_or_else(|_|"".to_string()), &data.characters)),
                        Button::new("import json")
                    ).clicked() {
                        match &mut data.character_code_edit_mode {
                            CodeEditMode::Export => {data.character_code_edit_mode = CodeEditMode::Importing("".to_string());}
                            CodeEditMode::Importing(s) => {
                                let character : (Character<StatIndexType>, Vec<ConcreteStatChange>) = serde_json::from_str(s).unwrap();
                                data.characters.insert(character.0.name.clone(), character);
                                s.clear();
                             }
                        }
                    }

                    let ui = &mut uis[0];
                    ScrollArea::vertical().show_rows(
                        ui,
                        ui.text_style_height(&egui::TextStyle::Body),
                        data.characters.len(),
                        |ui, range| {
                            for name in data
                                .characters
                                .keys()
                                .take(range.end)
                                .skip(range.start)
                            {
                                ui.selectable_value(
                                    &mut data.selected_character,
                                    name.to_owned(),
                                    name
                                );
                            }
                        }
                    );

                    let ui = &mut uis[2];
                    match &mut data.character_code_edit_mode {
                        CodeEditMode::Export => {
                            let copied_export = extract_character(data).unwrap_or_default();
                            ui.add(
                                TextEdit::multiline(&mut copied_export.as_str())
                                    .code_editor()
                                    .desired_width(0.0)
                            );
                        },
                        CodeEditMode::Importing(s) => {
                            ui.label("Paste the json here and then confirm by clicking \"import json\" again:");
                            ui.add(
                                TextEdit::multiline(s)
                                    .code_editor()
                                    .desired_width(0.0)
                            );
                        },
                    }
                });
            });
        if let Some(mut renamed) = std::mem::take(&mut data.renamed_character) {
            egui::Window::new("Renaming Character")
                .collapsible(false)
                .fixed_rect(inner_rect.unwrap().response.rect)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Character name: ");
                        ui.text_edit_singleline(&mut renamed.0.name);
                    });
                    if ui
                        .add_enabled(
                            check_legal_name(&renamed.0.name, &data.characters),
                            Button::new("confirm")
                        )
                        .clicked()
                    {
                        data.characters.insert(renamed.0.name.clone(), renamed);
                    }
                    else {
                        data.renamed_character = Some(renamed);
                    }
                });
        }
    }

    fn promotion_manager(data : &mut GameData, ctx : &egui::Context) {
        let inner_rect = egui::Window::new("Promotion Manager")
            .collapsible(data.renamed_promotion.is_none())
            .show(ctx, |ui| {
                ui.set_enabled(data.renamed_promotion.is_none());
                ui.columns(3, |uis| {
                    let ui = &mut uis[1];

                    ui.add_enabled_ui(
                        data.promotions.contains_key(&data.selected_promotion),
                        |ui| {
                            if ui.button("delete").clicked() {
                                data.promotions.remove(&data.selected_promotion);
                            }
                            if ui.button("rename").clicked() {
                                data.renamed_promotion =
                                    data.promotions.remove(&data.selected_promotion);
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                if ui.button("copy to clipboard").clicked() {
                                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                        let _best_effort = clipboard.set_text(
                                            serde_json::to_string(
                                                &data.promotions.get(&data.selected_promotion)
                                            )
                                            .unwrap()
                                        );
                                    }
                                }
                            }
                        }
                    );

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let mut clipboard_copied_promotion : Option<Character<StatIndexType>> = None;

                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            if let Ok(text) = clipboard.get_text() {
                                if let Ok(parse) =
                                    serde_json::from_str::<Character<StatIndexType>>(&text)
                                {
                                    if !data.promotions.contains_key(&parse.name) {
                                        clipboard_copied_promotion = Some(parse);
                                    }
                                }
                            }
                            ui.add_enabled_ui(clipboard_copied_promotion.is_some(), |ui| {
                                if ui.button("import from clipboard").clicked() {
                                    if let Some(clipboard_copied_promotion) =
                                        clipboard_copied_promotion
                                    {
                                        data.promotions.insert(
                                            clipboard_copied_promotion.name.clone(),
                                            clipboard_copied_promotion
                                        );
                                    }
                                }
                            });
                        }
                    }

                    if ui.add_enabled(
                        data.promotion_code_edit_mode != CodeEditMode::Export,
                        Button::new("export json")
                    ).clicked() {
                        data.promotion_code_edit_mode = CodeEditMode::Export;
                    }
                    if ui.add_enabled(
                        matches!(data.promotion_code_edit_mode, CodeEditMode::Export)
                            || matches!(&data.promotion_code_edit_mode, CodeEditMode::Importing(s)
                             if check_legal_name(&serde_json::from_str(s)
                             .unwrap_or(StatIndexType::new_default_character(data.game_option).name), &data.promotions)),
                        Button::new("import json")
                    ).clicked() {
                        match &mut data.promotion_code_edit_mode {
                            CodeEditMode::Export => {data.promotion_code_edit_mode = CodeEditMode::Importing("".to_string());}
                            CodeEditMode::Importing(s) => {
                                let promotion : Character<StatIndexType> = serde_json::from_str(s).unwrap();
                                data.promotions.insert(promotion.name.clone(), promotion);
                                s.clear();
                             }
                        }
                    }

                    let ui = &mut uis[0];
                    ScrollArea::vertical().show_rows(
                        ui,
                        ui.text_style_height(&egui::TextStyle::Body),
                        data.promotions.len(),
                        |ui, range| {
                            for name in data
                                .promotions
                                .keys()
                                .take(range.end)
                                .skip(range.start)
                            {
                                ui.selectable_value(
                                    &mut data.selected_promotion,
                                    name.to_owned(),
                                    name
                                );
                            }
                        }
                    );

                    let ui = &mut uis[2];
                    match &mut data.promotion_code_edit_mode {
                        CodeEditMode::Export => {
                            let copied_export = extract_promotion(data).unwrap_or_default();
                            ui.add(
                                TextEdit::multiline(&mut copied_export.as_str())
                                    .code_editor()
                                    .desired_width(0.0)
                            );
                        },
                        CodeEditMode::Importing(s) => {
                            ui.label("Paste the json here and then confirm by clicking \"import json\" again:");
                            ui.add(
                                TextEdit::multiline(s)
                                    .code_editor()
                                    .desired_width(0.0)
                            );
                        },
                    }
                });
            });
        if let Some(mut renamed) = std::mem::take(&mut data.renamed_promotion) {
            egui::Window::new("Renaming Promotion")
                .collapsible(false)
                .fixed_rect(inner_rect.unwrap().response.rect)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Promotion name: ");
                        ui.text_edit_singleline(&mut renamed.name);
                    });
                    if ui
                        .add_enabled(
                            check_legal_name(&renamed.name, &data.promotions),
                            Button::new("confirm")
                        )
                        .clicked()
                    {
                        data.promotions.insert(renamed.name.clone(), renamed);
                    }
                    else {
                        data.renamed_promotion = Some(renamed);
                    }
                });
        }
    }
}

fn check_legal_name<T>(name : &str, data : &BTreeMap<String, T>) -> bool {
    !name.is_empty()
        && !data
            .iter()
            .map(|(name, _data)| name.to_lowercase())
            .contains(&name.to_lowercase())
}

fn extract_promotion(data : &GameData) -> Option<String> {
    serde_json::to_string(data.promotions.get(&data.selected_promotion)?).ok()
}

fn extract_character(data : &GameData) -> Option<String> {
    serde_json::to_string(data.characters.get(&data.selected_character)?).ok()
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
