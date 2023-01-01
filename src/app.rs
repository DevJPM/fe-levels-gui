use std::{
    collections::{BTreeMap, HashMap},
    fmt::{self, Display},
    str::FromStr,
    sync::Arc
};

use eframe::epaint;
use egui::{
    vec2, Button, Context, CursorIcon, Grid, Id, InnerResponse, Label, NumExt, Rect, ScrollArea,
    Sense, Shape, TextEdit, Ui, Vec2
};
use fe_levels::{BlankAvoidance, Character, StatChange, StatType};
use itertools::Itertools;

use rand::random;
use serde::{Deserialize, Serialize};

use self::{
    plotter::PlotterManager,
    sit::{template_stat, StatIndexType}
};

mod plotter;
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
    templates : Vec<ConcreteStatChange>,
    progression : Vec<ConcreteStatChange>,
    id : Id,
    queued_insertion : Option<(usize, ConcreteStatChange)>,
    promotion_selection_strategy : PromotionSelectionKind,
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
        templates : Default::default(),
        progression : Default::default(),
        id : Id::new(UsefulId::default()),
        queued_insertion : Default::default(),
        promotion_selection_strategy : Default::default(),
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

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
enum GbaFeStatChange {
    Promotion(Character<StatIndexType>),
    LevelUp,
    GrowthBooster,
    StatBooster(StatIndexType)
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Default)]
enum PromotionSelectionKind {
    LoadSavedPromotion,
    #[default]
    ManualPromotionEntry
}

impl GbaFeStatChange {
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
                        &mut context.promotion_selection_strategy,
                        PromotionSelectionKind::ManualPromotionEntry,
                        "Manual Promotion Entry"
                    );
                    ui.radio_value(
                        &mut context.promotion_selection_strategy,
                        PromotionSelectionKind::LoadSavedPromotion,
                        "Select Saved Promotion"
                    );
                });

                match context.promotion_selection_strategy {
                    PromotionSelectionKind::LoadSavedPromotion => {
                        ScrollArea::vertical().show_rows(
                            ui,
                            ui.text_style_height(&egui::TextStyle::Body),
                            context.progression.len(),
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
                                    check_legal_name(&promotion_gains.name, &context.promotions),
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

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
enum ConcreteStatChange {
    GbaFeStatChange(GbaFeStatChange)
}

impl ConcreteStatChange {
    fn compile(self) -> StatChange<StatIndexType> {
        match self {
            ConcreteStatChange::GbaFeStatChange(data) => data.compile()
        }
    }

    fn cheap_to_execute(&self) -> bool {
        match self {
            ConcreteStatChange::GbaFeStatChange(data) => data.cheap_to_execute()
        }
    }

    fn increases_level_counter(&self) -> bool {
        match self {
            ConcreteStatChange::GbaFeStatChange(data) => data.increases_level_counter()
        }
    }

    fn resets_level_counter(&self) -> bool {
        match self {
            ConcreteStatChange::GbaFeStatChange(data) => data.resets_level_counter()
        }
    }

    fn generate_templates(game_option : GameKind) -> Vec<Self> {
        match game_option {
            GameKind::GbaFe => vec![
                ConcreteStatChange::GbaFeStatChange(GbaFeStatChange::GrowthBooster),
                ConcreteStatChange::GbaFeStatChange(GbaFeStatChange::LevelUp),
                ConcreteStatChange::GbaFeStatChange(GbaFeStatChange::StatBooster(template_stat(
                    GameKind::GbaFe
                ))),
                ConcreteStatChange::GbaFeStatChange(GbaFeStatChange::Promotion(Character {
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
                    name : "".to_owned()
                })),
            ],
            GameKind::PoR => vec![]
        }
    }

    fn marking_worthy(&self) -> bool {
        match self {
            ConcreteStatChange::GbaFeStatChange(data) => data.marking_worthy()
        }
    }

    fn clarification_dialogue(self, context : &mut GameData, ui : &mut Ui) -> (Self, bool) {
        match self {
            ConcreteStatChange::GbaFeStatChange(data) => {
                let (data, ready) = data.clarification_dialogue(context, ui);
                (ConcreteStatChange::GbaFeStatChange(data), ready)
            }
        }
    }

    fn requires_clarification(&self) -> bool {
        match self {
            ConcreteStatChange::GbaFeStatChange(data) => data.requires_clarification()
        }
    }
}

impl fmt::Display for ConcreteStatChange {
    fn fmt(&self, f : &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConcreteStatChange::GbaFeStatChange(sc) => Display::fmt(sc, f)
        }
    }
}

pub fn drag_source(
    ui : &mut Ui,
    id : Id,
    keep_showing_original : bool,
    mut drag_handle : impl FnMut(&mut Ui),
    context_menu : Option<impl FnOnce(&mut Ui)>
) -> Option<Rect> {
    let is_being_dragged = ui.memory().is_being_dragged(id);

    if !is_being_dragged {
        let row_resp = ui.horizontal(|gg| {
            let u = gg.scope(drag_handle);

            // Check for drags:
            let response = gg.interact(u.response.rect, id, Sense::click_and_drag());

            if response.hovered() {
                gg.output().cursor_icon = CursorIcon::Grab;
            }

            if let Some(context_menu) = context_menu {
                response.context_menu(context_menu);
            }
        });

        return Some(row_resp.response.rect);
    }
    else {
        ui.output().cursor_icon = CursorIcon::Grabbing;

        if keep_showing_original {
            drag_handle(ui);
        }

        // Now we move the visuals of the body to where the mouse is.
        // Normally you need to decide a location for a widget first,
        // because otherwise that widget cannot interact with the mouse.
        // However, a dragged component cannot be interacted with anyway
        // (anything with `Order::Tooltip` always gets an empty [`Response`])
        // So this is fine!

        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            egui::Area::new("draggable_item")
                .interactable(false)
                .fixed_pos(pointer_pos)
                .show(ui.ctx(), drag_handle);
        }
    }

    None
}

fn drop_target<R>(
    ui : &mut Ui,
    is_being_dragged : bool,
    scroll_id : BuilderColumn,
    body : impl FnOnce(&mut Ui) -> R
) -> InnerResponse<R> {
    let margin = Vec2::splat(4.0);
    /*ScrollArea::vertical()
    .id_source(scroll_id)
    .auto_shrink([true, true])
    .show(ui, |ui| {*/
    // perhaps show_rows works better here?
    let outer_rect_bounds = ui.available_rect_before_wrap();
    let inner_rect = outer_rect_bounds.shrink2(margin);
    let where_to_put_background = ui.painter().add(Shape::Noop);

    let mut content_ui = ui.child_ui(inner_rect, *ui.layout());

    let ret = body(&mut content_ui);
    let outer_rect = Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);
    let (rect, response) = ui.allocate_at_least(outer_rect.size(), Sense::hover());

    let style = if is_being_dragged && response.hovered() {
        ui.visuals().widgets.active
    }
    else {
        ui.visuals().widgets.inactive
    };

    let fill = style.bg_fill;
    let stroke = style.bg_stroke;

    ui.painter().set(
        where_to_put_background,
        epaint::RectShape {
            rounding : style.rounding,
            fill,
            stroke,
            rect
        }
    );

    InnerResponse::new(ret, response)
    /* }) */
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
struct DndIntState<T : Clone + Send + Sync + 'static> {
    target_row_id : Option<usize>,

    drop_col : Option<BuilderColumn>,

    source_col_row : Option<(BuilderColumn, usize)>,

    dragged_object : Option<T>
}

impl<T : Clone + Send + Sync + 'static> Default for DndIntState<T> {
    fn default() -> Self {
        Self {
            target_row_id : Default::default(),
            drop_col : Default::default(),
            source_col_row : Default::default(),
            dragged_object : Default::default()
        }
    }
}

impl<T : Clone + Send + Sync + 'static> DndIntState<T> {
    pub fn load(ctx : &Context, id : Id) -> Option<Self> { ctx.data().get_temp(id) }

    pub fn store(self, ctx : &Context, id : Id) { ctx.data().insert_temp(id, self); }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum BuilderColumn {
    Levels,
    Templates
}

fn extract_promotion(data : &GameData) -> Option<String> {
    serde_json::to_string(data.promotions.get(&data.selected_promotion)?).ok()
}

fn extract_character(data : &GameData) -> Option<String> {
    serde_json::to_string(data.characters.get(&data.selected_character)?).ok()
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
                                data.progression = data.characters.get(&data.selected_character).unwrap().1.clone();
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
                            let copied_export = extract_character(data).unwrap_or_else(||"".to_string());
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
                            let copied_export = extract_promotion(data).unwrap_or_else(||"".to_string());
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

    // TODO: make the left side scrollable
    fn character_progression_builder(data : &mut GameData, ctx : &egui::Context) {
        if data.templates != ConcreteStatChange::generate_templates(data.game_option) {
            data.templates = ConcreteStatChange::generate_templates(data.game_option);
        }

        let builder_rect = egui::Window::new("Character Progression Builder")
            .collapsible(data.queued_insertion.is_none())
            .show(ctx, |ui| {
                ui.set_enabled(data.queued_insertion.is_none());

                let mut container_rect = None;
                let mut row_rect = None;

                let progression = &mut data.progression;
                let templates = &mut data.templates;

                let mut state : DndIntState<ConcreteStatChange> =
                    DndIntState::load(ui.ctx(), data.id).unwrap_or_default();

                let drag_target_row_position = &mut state.target_row_id;
                let source_col_row = &mut state.source_col_row;
                let drop_col = &mut state.drop_col;
                let dragged_object = &mut state.dragged_object;

                ui.label(
                    "The index (#2) indicates the numerical x-axis coordinate for the result of \
                     this stat change."
                );

                if ui.button("clear all").clicked() {
                    progression.clear();
                }

                ui.columns(2, |uis| {
                    let mut render_column = |col_idx,
                                             ui,
                                             column : Vec<ConcreteStatChange>,
                                             drag_handler : &mut dyn FnMut(
                        &mut Ui,
                        &ConcreteStatChange,
                        usize
                    ),
                                             mut context_handler : Option<
                        &mut dyn FnMut(&mut Ui, &ConcreteStatChange, usize)
                    >| {
                        let this_col_is_dest = drop_col.map(|x| x == col_idx).unwrap_or(false);

                        let response = drop_target(ui, this_col_is_dest, col_idx, |ui| {
                            //
                            ui.set_min_size(vec2(64.0, 100.0));
                            for (row_idx, item) in column.iter().enumerate() {
                                let item_id = data.id.with(col_idx).with(row_idx);

                                // this handles the preview label for non tail end insertions
                                if source_col_row.is_some()
                                    && *drag_target_row_position == Some(row_idx)
                                    && drop_col
                                        .map(|col| col == col_idx && col == BuilderColumn::Levels)
                                        .unwrap_or(false)
                                    && dragged_object.is_some()
                                {
                                    ui.add(Label::new(
                                        dragged_object.as_ref().unwrap().to_string()
                                    ));
                                }

                                let c_row_size_rect = drag_source(
                                    ui,
                                    item_id,
                                    col_idx == BuilderColumn::Templates,
                                    |ui| {
                                        drag_handler(ui, item, row_idx);
                                    },
                                    context_handler.as_mut().map(|f| {
                                        |ui : &mut Ui| {
                                            f(ui, item, row_idx);
                                        }
                                    })
                                );

                                if c_row_size_rect.is_some() {
                                    row_rect = c_row_size_rect;
                                }

                                if ui.memory().is_being_dragged(item_id) {
                                    *source_col_row = Some((col_idx, row_idx));
                                    *dragged_object = Some(item.clone());
                                }
                            }

                            // this handles the preview label for tail-end insertions
                            if source_col_row.is_some()
                                && drag_target_row_position
                                    .map(|x| x >= column.len())
                                    .unwrap_or(false)
                                && drop_col
                                    .map(|col| col == col_idx && col == BuilderColumn::Levels)
                                    .unwrap_or(false)
                                && dragged_object.is_some()
                            {
                                ui.add(Label::new(dragged_object.as_ref().unwrap().to_string()));
                            }
                        })
                        .response;

                        let is_being_dragged = source_col_row.is_some();

                        if is_being_dragged && response.hovered() {
                            *drop_col = Some(col_idx);
                            container_rect = Some(response.rect);
                        }
                    };
                    if let [ui1, ui2] = uis {
                        let copy = progression.clone();
                        render_column(
                            BuilderColumn::Levels,
                            ui1,
                            progression.clone(),
                            &mut |ui, item, row_idx| {
                                if item.increases_level_counter() {
                                    ui.label(format!(
                                        "(#{}) {item} to {}",
                                        row_idx + 2,
                                        find_row_level(&copy, row_idx).unwrap()
                                    ));
                                }
                                else {
                                    ui.label(format!("(#{}) {item}", row_idx + 2));
                                }
                            },
                            Some(&mut |ui, item, row_idx| {
                                if ui
                                    .add_enabled(
                                        item.requires_clarification(),
                                        Button::new("reconfigure")
                                    )
                                    .clicked()
                                {
                                    let item = progression.remove(row_idx);
                                    data.queued_insertion = Some((row_idx, item));
                                    ui.close_menu();
                                }
                            })
                        );
                        render_column(
                            BuilderColumn::Templates,
                            ui2,
                            templates.clone(),
                            &mut |ui, item, _row_idx| {
                                ui.label(item.to_string());
                            },
                            None
                        );
                    }
                });

                if let (Some(_drop_col), Some(row_rect), Some(container_rect)) =
                    (*drop_col, row_rect, container_rect)
                {
                    if ui.memory().is_anything_being_dragged() {
                        let pos = ui.input().pointer.hover_pos();

                        let row_rectr = row_rect.size();

                        let offset = pos.unwrap() - container_rect.min;

                        let drag_position =
                            ((offset.y - row_rectr.y / 2.) / row_rectr.y).round() as usize;
                        // .at_most(self.columns[drop_col].len().saturating_sub(1));

                        *drag_target_row_position = Some(drag_position);
                    }
                    else {
                        *drag_target_row_position = None;
                    }
                }
                else {
                    *drag_target_row_position = None;
                }

                if let Some((source_col, source_row)) = *source_col_row {
                    if let Some(drop_col) = *drop_col {
                        //
                        if ui.input().pointer.any_released() {
                            // do the drop:

                            if let Some(drag_target_row_position) = drag_target_row_position {
                                let item = match source_col {
                                    BuilderColumn::Levels => progression.remove(source_row),
                                    BuilderColumn::Templates => templates[source_row].clone()
                                };

                                if drop_col == BuilderColumn::Levels {
                                    let insert_index =
                                        drag_target_row_position.at_most(progression.len());
                                    match source_col {
                                        BuilderColumn::Levels => {
                                            progression.insert(insert_index, item)
                                        },
                                        BuilderColumn::Templates => {
                                            data.queued_insertion = Some((insert_index, item))
                                        },
                                    }
                                }
                            }
                        }
                    }
                }

                if ui.input().pointer.any_released() {
                    *source_col_row = None;
                    *drop_col = None;
                    *dragged_object = None;
                    *drag_target_row_position = None;
                }

                state.store(ui.ctx(), data.id);
                ui.min_rect()
            });

        if let Some((index, queued_insertion)) = std::mem::take(&mut data.queued_insertion) {
            egui::Window::new("Specify Details")
                .collapsible(false)
                .fixed_rect(builder_rect.unwrap().inner.unwrap())
                .show(ctx, |ui| {
                    ctx.move_to_top(ui.layer_id());
                    let (stat_change, ready) = queued_insertion.clarification_dialogue(data, ui);
                    if ready {
                        data.progression.insert(index, stat_change);
                    }
                    else {
                        data.queued_insertion = Some((index, stat_change))
                    }
                });
        }
    }
}

fn find_row_level(progression : &[ConcreteStatChange], row_idx : usize) -> Option<usize> {
    let mut current_level = 1;
    for (row, csc) in progression.iter().enumerate() {
        if csc.increases_level_counter() {
            current_level += 1;
        }
        if csc.resets_level_counter() {
            current_level = 1;
        }
        if row == row_idx {
            return Some(current_level);
        }
    }
    None
}
fn check_legal_name<T>(name : &str, data : &BTreeMap<String, T>) -> bool {
    !name.is_empty()
        && !data
            .iter()
            .map(|(name, _data)| name.to_lowercase())
            .contains(&name.to_lowercase())
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
        Self::character_progression_builder(
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
