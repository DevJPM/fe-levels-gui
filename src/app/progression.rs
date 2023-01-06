use std::{
    fmt,
    ops::{Deref, DerefMut}
};

use eframe::epaint;
use egui::{
    vec2, Button, Context, CursorIcon, Id, InnerResponse, Label, NumExt, Rect, Sense, Shape, Ui,
    Vec2
};
use fe_levels::StatChange;
use serde::{Deserialize, Serialize};

use self::gba::GbaFeStatChange;

use super::{sit::StatIndexType, GameData, GameKind, UsefulId};

mod gba;

#[derive(Deserialize, Serialize, Default)]
pub struct ProgressionManager {
    templates : Vec<ConcreteStatChange>,
    progression : Vec<ConcreteStatChange>,
    id : UsefulId,
    queued_insertion : Option<(usize, ConcreteStatChange)>,
    promotion_selection_strategy : PromotionSelectionKind
}

impl Deref for ProgressionManager {
    type Target = Vec<ConcreteStatChange>;
    fn deref(&self) -> &Self::Target { &self.progression }
}

impl DerefMut for ProgressionManager {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.progression }
}

impl ProgressionManager {
    fn id(&self) -> Id { Id::new(self.id) }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConcreteStatChange {
    GbaFeStatChange(GbaFeStatChange)
}

pub trait UsefulStatChange: fmt::Display {
    fn compile(self) -> StatChange<StatIndexType>;
    fn cheap_to_execute(&self) -> bool;
    fn increases_level_counter(&self) -> bool;
    fn resets_level_counter(&self) -> bool;
    fn generate_templates(game_option : GameKind) -> Vec<Self>
    where
        Self : Sized;
    fn marking_worthy(&self) -> bool;
    /// true on return indicates the user confirmed the result and it should be
    /// seen as final
    fn clarification_dialogue(self, context : &mut GameData, ui : &mut Ui) -> (Self, bool)
    where
        Self : Sized;
    fn requires_clarification(&self) -> bool;
}

impl UsefulStatChange for ConcreteStatChange {
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
            GameKind::GbaFe => GbaFeStatChange::generate_templates(GameKind::GbaFe)
                .into_iter()
                .map(ConcreteStatChange::GbaFeStatChange)
                .collect(),
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
            ConcreteStatChange::GbaFeStatChange(sc) => fmt::Display::fmt(sc, f)
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PromotionSelectionKind {
    LoadSavedPromotion,
    #[default]
    ManualPromotionEntry
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
    _scroll_id : BuilderColumn,
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

// TODO: make the left side scrollable
pub fn character_progression_builder(data : &mut GameData, ctx : &egui::Context) {
    if data.progression.templates != ConcreteStatChange::generate_templates(data.game_option) {
        data.progression.templates = ConcreteStatChange::generate_templates(data.game_option);
    }

    let builder_rect = egui::Window::new("Character Progression Builder")
        .collapsible(data.progression.queued_insertion.is_none())
        .show(ctx, |ui| {
            ui.set_enabled(data.progression.queued_insertion.is_none());

            let mut container_rect = None;
            let mut row_rect = None;

            let mut state : DndIntState<ConcreteStatChange> =
                DndIntState::load(ui.ctx(), data.progression.id()).unwrap_or_default();

            let drag_target_row_position = &mut state.target_row_id;
            let source_col_row = &mut state.source_col_row;
            let drop_col = &mut state.drop_col;
            let dragged_object = &mut state.dragged_object;

            ui.label(
                "The index (#2) indicates the numerical x-axis coordinate for the result of this \
                 stat change."
            );

            if ui.button("clear all").clicked() {
                data.progression.progression.clear();
            }

            ui.columns(2, |uis| {
                let id = data.progression.id();
                let mut render_column =
                    |col_idx,
                     ui,
                     column : Vec<ConcreteStatChange>,
                     drag_handler : &mut dyn FnMut(&mut Ui, &ConcreteStatChange, usize),
                     mut context_handler : Option<
                        &mut dyn FnMut(&mut Ui, &ConcreteStatChange, usize)
                    >| {
                        let this_col_is_dest = drop_col.map(|x| x == col_idx).unwrap_or(false);

                        let response = drop_target(ui, this_col_is_dest, col_idx, |ui| {
                            //
                            ui.set_min_size(vec2(64.0, 100.0));
                            for (row_idx, item) in column.iter().enumerate() {
                                let item_id = id.with(col_idx).with(row_idx);

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
                    let copy = (data.progression.progression).clone();
                    render_column(
                        BuilderColumn::Levels,
                        ui1,
                        data.progression.progression.clone(),
                        &mut |ui, item, row_idx| {
                            if item.increases_level_counter() {
                                ui.label(format!(
                                    "(#{}) {item} to {}",
                                    row_idx + 2,
                                    find_row_level(data.character.level, &copy, row_idx).unwrap()
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
                                let item = data.progression.progression.remove(row_idx);
                                data.progression.queued_insertion = Some((row_idx, item));
                                ui.close_menu();
                            }
                        })
                    );
                    render_column(
                        BuilderColumn::Templates,
                        ui2,
                        (data.progression.templates).clone(),
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
                                BuilderColumn::Levels => {
                                    data.progression.progression.remove(source_row)
                                },
                                BuilderColumn::Templates => {
                                    (&mut data.progression.templates)[source_row].clone()
                                },
                            };

                            if drop_col == BuilderColumn::Levels {
                                let insert_index = drag_target_row_position
                                    .at_most(data.progression.progression.len());
                                match source_col {
                                    BuilderColumn::Levels => {
                                        data.progression.progression.insert(insert_index, item)
                                    },
                                    BuilderColumn::Templates => {
                                        data.progression.queued_insertion =
                                            Some((insert_index, item))
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

            state.store(ui.ctx(), data.progression.id());
            ui.min_rect()
        });

    if let Some((index, queued_insertion)) = std::mem::take(&mut data.progression.queued_insertion)
    {
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
                    data.progression.queued_insertion = Some((index, stat_change))
                }
            });
    }
}

fn find_row_level(
    base_level : usize,
    progression : &[ConcreteStatChange],
    row_idx : usize
) -> Option<usize> {
    let mut current_level = base_level;
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
