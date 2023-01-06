use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut}
};

use egui::{Button, Rect, ScrollArea, TextEdit, Ui};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, Default)]
enum CodeEditMode {
    #[default]
    Export,
    Importing(String)
}

#[derive(Serialize, Deserialize)]
pub struct DataManaged<V> {
    data : BTreeMap<String, V>,
    selected : String,
    renamed : Option<(String, V)>,
    edit_mode : CodeEditMode
}

impl<V> Default for DataManaged<V> {
    fn default() -> Self {
        Self {
            data : Default::default(),
            selected : Default::default(),
            renamed : Default::default(),
            edit_mode : Default::default()
        }
    }
}

impl<V> Deref for DataManaged<V> {
    type Target = BTreeMap<String, V>;

    fn deref(&self) -> &Self::Target { &self.data }
}

impl<V> DerefMut for DataManaged<V> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.data }
}

impl<V> DataManaged<V> {
    pub fn selected(&self) -> Option<&V> { self.data.get(&self.selected) }

    pub fn check_legal_name(&self, name : &str) -> bool {
        !name.is_empty()
            && !self
                .data
                .iter()
                .map(|(name, _data)| name.to_lowercase())
                .contains(&name.to_lowercase())
    }
}

impl<V : Serialize + for<'a> Deserialize<'a>> DataManaged<V> {
    fn extract(&self) -> Option<String> {
        serde_json::to_string(self.data.get(&self.selected)?).ok()
    }

    pub fn management_dialogue(
        &mut self,
        ctx : &egui::Context,
        external_modal_open : bool,
        window_title : &str,
        deserialize_name : impl Fn(&V) -> String,
        buttons : impl FnOnce(&mut Ui, &mut Self)
    ) -> Option<Rect> {
        let modal_open = external_modal_open || self.renamed.is_some();
        let window_response = egui::Window::new(window_title)
            .collapsible(!modal_open)
            .show(ctx, |ui| {
                ui.set_enabled(!modal_open);
                let out_rect = ui.available_rect_before_wrap();
                ui.columns(3, |uis| {
                    let ui = &mut uis[1];

                    buttons(ui, self);

                    ui.add_enabled_ui(self.data.contains_key(&self.selected), |ui| {
                        if ui.button("delete").clicked() {
                            self.data.remove(&self.selected);
                        }
                        if ui.button("rename").clicked() {
                            self.renamed = self
                                .data
                                .remove(&self.selected)
                                .map(|v| (self.selected.clone(), v));
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            if ui.button("copy to clipboard").clicked() {
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    let _best_effort = clipboard.set_text(
                                        serde_json::to_string(
                                            &self.data.get(&self.selected).unwrap()
                                        )
                                        .unwrap()
                                    );
                                }
                            }
                        }
                    });

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let mut clipboard_copied_promotion : Option<V> = None;

                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            if let Ok(text) = clipboard.get_text() {
                                if let Ok(parse) = serde_json::from_str::<V>(&text) {
                                    if !self.data.contains_key(&deserialize_name(&parse)) {
                                        clipboard_copied_promotion = Some(parse);
                                    }
                                }
                            }
                            ui.add_enabled_ui(clipboard_copied_promotion.is_some(), |ui| {
                                if ui.button("import from clipboard").clicked() {
                                    if let Some(clipboard_copied_promotion) =
                                        clipboard_copied_promotion
                                    {
                                        self.data.insert(
                                            deserialize_name(&clipboard_copied_promotion),
                                            clipboard_copied_promotion
                                        );
                                    }
                                }
                            });
                        }
                    }

                    if ui
                        .add_enabled(
                            self.edit_mode != CodeEditMode::Export,
                            Button::new("export json")
                        )
                        .clicked()
                    {
                        self.edit_mode = CodeEditMode::Export;
                    }

                    if ui
                        .add_enabled(
                            matches!(self.edit_mode, CodeEditMode::Export)
                                || self.check_importable_text(&deserialize_name),
                            Button::new("import json")
                        )
                        .clicked()
                    {
                        match &mut self.edit_mode {
                            CodeEditMode::Export => {
                                self.edit_mode = CodeEditMode::Importing("".to_string());
                            },
                            CodeEditMode::Importing(s) => {
                                let read_value : V = serde_json::from_str(s).unwrap();
                                self.data.insert(deserialize_name(&read_value), read_value);
                                s.clear();
                            }
                        }
                    }

                    let ui = &mut uis[0];
                    ScrollArea::vertical().show_rows(
                        ui,
                        ui.text_style_height(&egui::TextStyle::Body),
                        self.data.len(),
                        |ui, range| {
                            for name in self.data.keys().take(range.end).skip(range.start) {
                                ui.selectable_value(&mut self.selected, name.to_owned(), name);
                            }
                        }
                    );

                    let ui = &mut uis[2];
                    match &mut self.edit_mode {
                        CodeEditMode::Export => {
                            let copied_export = self.extract().unwrap_or_default();
                            ui.add(
                                TextEdit::multiline(&mut copied_export.as_str())
                                    .code_editor()
                                    .desired_width(0.0)
                            );
                        },
                        CodeEditMode::Importing(s) => {
                            ui.label(
                                "Paste the json here and then confirm by clicking \"import json\" \
                                 again:"
                            );
                            ui.add(TextEdit::multiline(s).code_editor().desired_width(0.0));
                        }
                    }
                });
                out_rect
            });
        let modal_rect = window_response.map(|response| response.response.rect);

        let copy_rect = modal_rect.clone();

        if let Some((mut name, item)) = std::mem::take(&mut self.renamed) {
            egui::Window::new("Renaming Promotion")
                .collapsible(false)
                .fixed_rect(modal_rect.unwrap())
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Promotion name: ");
                        ui.text_edit_singleline(&mut name);
                    });
                    if ui
                        .add_enabled(self.check_legal_name(&name), Button::new("confirm"))
                        .clicked()
                    {
                        self.data.insert(name, item);
                    }
                    else {
                        self.renamed = Some((name, item));
                    }
                });
        }

        copy_rect
    }

    fn check_importable_text(&self, deserialize_name : &impl Fn(&V) -> String) -> bool {
        if let CodeEditMode::Importing(s) = &self.edit_mode {
            if let Ok(parsed) = serde_json::from_str(s) {
                return self.check_legal_name(&deserialize_name(&parsed));
            }
        }
        false
    }
}
