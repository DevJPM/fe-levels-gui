use std::{
    borrow::Borrow,
    collections::{BTreeMap, HashMap},
    fmt::{self, Display},
    str::FromStr,
    thread::JoinHandle
};

use cached::proc_macro::cached;
use egui::{
    plot::{Bar, BarChart, BoxElem, BoxSpread, Legend, Line, Plot, PlotPoint, PlotPoints},
    reset_button_with,
    widgets::plot::BoxPlot,
    Slider, Ui
};
use fe_levels::{BlankAvoidance, Character, Stat, StatChange, StatType};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

type CompleteData = Vec<BTreeMap<StatIndexType, BTreeMap<fe_levels::StatType, f64>>>;
type StatIndexType = String;

#[derive(Default)]
enum ComputeState {
    #[default]
    Idle,
    Processing(JoinHandle<CompleteData>),
    Done(CompleteData)
}

#[derive(PartialEq, Default, Deserialize, Serialize)]
enum ChartKind {
    IntraLevelDist,
    InterLevelDist,
    #[default]
    BoxPlots
}

#[derive(PartialEq, Default, Deserialize, Serialize, Hash, Eq, Clone, Copy, Debug)]
enum GameKind {
    #[default]
    GbaFe,
    PoR
}

impl fmt::Display for ChartKind {
    fn fmt(&self, f : &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ChartKind::IntraLevelDist => "Focus One Level",
                ChartKind::InterLevelDist => "Show Multiple Levels",
                ChartKind::BoxPlots => "Box Plot"
            }
        )
    }
}

#[derive(PartialEq, Default, Deserialize, Serialize)]
enum ReductionKind {
    #[default]
    AverageReduction,
    BenchmarkReduction
}

#[derive(PartialEq, Default, Deserialize, Serialize)]
enum IntraLevelDetails {
    #[default]
    DensityData,
    CumulativeData
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Deserialize, Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct FeLevelGui {
    #[serde(skip)]
    derived_data : ComputeState,

    chart_type : ChartKind,

    reduction_option : ReductionKind,

    intra_level_option : IntraLevelDetails,

    game_option : GameKind,

    selected_stat : HashMap<GameKind, StatIndexType>,

    character : HashMap<GameKind, Character<StatIndexType>>,

    need_recalc : bool,

    benchmark : StatType,

    box_range : u8,

    inspected_level : usize
}

const GBA_FE_ORDER : [&str; 7] = ["HP", "Atk", "Skl", "Spd", "Lck", "Def", "Res"];
const POR_ORDER : [&str; 8] = ["HP", "Str", "Mag", "SKl", "Spd", "Lck", "Def", "Res"];

fn generate_default_character(game : GameKind) -> Character<StatIndexType> {
    let stat_list = look_up_iteration_order(game);
    Character {
        stats : stat_list
            .into_iter()
            .map(|s| {
                (
                    s.to_string(),
                    Stat {
                        base : 5,
                        cap : 20,
                        growth : 50,
                        value : 5
                    }
                )
            })
            .collect(),
        name : Default::default()
    }
}

fn look_up_iteration_order(game : GameKind) -> Vec<&'static str> {
    let stat_list = match game {
        GameKind::GbaFe => Vec::from(GBA_FE_ORDER),
        GameKind::PoR => Vec::from(POR_ORDER)
    };
    stat_list
}

fn numerical_text_box<T : Display + FromStr>(ui : &mut Ui, value : &mut T) {
    let mut text = value.to_string();
    ui.text_edit_singleline(&mut text);
    if let Ok(parsed) = str::parse(&text) {
        *value = parsed;
    }
    // do not write the result back / do anything in case of a bad parse
}

#[cached(size = 100)]
fn compute(character : Character<StatIndexType>, num_samples : Option<u64>) -> CompleteData {
    fe_levels::generate_histograms(
        &[
            StatChange::LevelUp {
                temporary_growth_override : None,
                blank_avoidance : BlankAvoidance::NoAvoidance
            },
            StatChange::LevelUp {
                temporary_growth_override : None,
                blank_avoidance : BlankAvoidance::NoAvoidance
            },
            StatChange::LevelUp {
                temporary_growth_override : None,
                blank_avoidance : BlankAvoidance::NoAvoidance
            },
            StatChange::LevelUp {
                temporary_growth_override : None,
                blank_avoidance : BlankAvoidance::NoAvoidance
            },
            StatChange::LevelUp {
                temporary_growth_override : None,
                blank_avoidance : BlankAvoidance::NoAvoidance
            }
        ],
        &character,
        num_samples
    )
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

    // class manager?

    fn character_builder(&mut self, ctx : &egui::Context) {
        egui::Window::new("Character Builder").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name: ");
                ui.text_edit_singleline(
                    &mut self
                        .character
                        .entry(self.game_option)
                        .or_insert_with(|| generate_default_character(self.game_option))
                        .name
                );
            });
            egui::Grid::new("Character Builder Table").show(ui, |ui| {
                ui.label("Stat");
                ui.label("Base");
                ui.label("Cap");
                ui.label("Growth");
                ui.end_row();

                self.character
                    .entry(self.game_option)
                    .or_insert_with(|| generate_default_character(self.game_option))
                    .stats
                    .iter_mut()
                    .sorted_by_cached_key(|(key, _value)| {
                        look_up_iteration_order(self.game_option)
                            .into_iter()
                            .position(|s| &s == key)
                            .expect(&format!(
                                "Failed to find {key} for game {:?}.",
                                self.game_option
                            ))
                    })
                    .for_each(|(key, stat)| {
                        let copy = stat.clone();
                        ui.label(key);
                        ui.add(egui::Slider::new(&mut stat.base, 0..=stat.cap));
                        stat.value = stat.base;
                        numerical_text_box(ui, &mut stat.cap);
                        numerical_text_box(ui, &mut stat.growth);
                        self.need_recalc = self.need_recalc || (copy != *stat);
                        ui.end_row()
                    });
            });
        });
    }

    fn data_manager(ctx : &egui::Context) {
        egui::Window::new("Data Manager").show(ctx, |ui| {
            ui.label("Windows can be moved by dragging them.");
            ui.label("They are automatically sized based on contents.");
            ui.label("You can turn on resizing and scrolling if you like.");
            ui.label("You would normally chose either panels OR windows.");
        });
    }

    fn character_progression_builder(ctx : &egui::Context) {
        egui::Window::new("Character Progression Builder").show(ctx, |ui| {
            ui.label("Windows can be moved by dragging them.");
            ui.label("They are automatically sized based on contents.");
            ui.label("You can turn on resizing and scrolling if you like.");
            ui.label("You would normally chose either panels OR windows.");
        });
    }

    fn data_plotting_window(&mut self, ctx : &egui::Context) {
        egui::Window::new("Data Plotter").show(ctx, |ui| {
            let extracted_handle = std::mem::take(&mut self.derived_data);

            match extracted_handle {
                ComputeState::Processing(handle) if !handle.is_finished() => {
                    self.derived_data = ComputeState::Processing(handle);
                    ui.spinner();
                },
                ComputeState::Processing(handle) if handle.is_finished() => {
                    let actual_data = handle.join().unwrap();

                    self.actual_data_display(ui, &actual_data);
                    self.derived_data = ComputeState::Done(actual_data);
                },
                ComputeState::Done(actual_data) if !self.need_recalc => {
                    self.actual_data_display(ui, &actual_data);
                    self.derived_data = ComputeState::Done(actual_data);
                },
                ComputeState::Idle => {
                    let character = self.character.get(&self.game_option).unwrap().clone();
                    self.derived_data = ComputeState::Processing(std::thread::spawn(move || {
                        compute(character, Some(1u64 << 20))
                    }));
                    ui.spinner();
                    self.need_recalc = false;
                },
                ComputeState::Done(_) if self.need_recalc => {
                    let character = self.character.get(&self.game_option).unwrap().clone();
                    self.derived_data = ComputeState::Processing(std::thread::spawn(move || {
                        compute(character, Some(1u64 << 20))
                    }));
                    ui.spinner();
                    self.need_recalc = false;
                },
                _ => unreachable!()
            }
        });
    }

    fn actual_data_display(&mut self, ui : &mut Ui, actual_data : &CompleteData) {
        ui.horizontal_top(|ui| {
            egui::containers::ComboBox::from_label("Data to Display")
                .selected_text(self.chart_type.to_string())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.chart_type,
                        ChartKind::IntraLevelDist,
                        ChartKind::IntraLevelDist.to_string()
                    );
                    ui.selectable_value(
                        &mut self.chart_type,
                        ChartKind::InterLevelDist,
                        ChartKind::InterLevelDist.to_string()
                    );
                    ui.selectable_value(
                        &mut self.chart_type,
                        ChartKind::BoxPlots,
                        ChartKind::BoxPlots.to_string()
                    );
                });
            match self.chart_type {
                ChartKind::IntraLevelDist => {
                    ui.radio_value(
                        &mut self.intra_level_option,
                        IntraLevelDetails::DensityData,
                        "Chance to hit the stat exactly"
                    );
                    ui.radio_value(
                        &mut self.intra_level_option,
                        IntraLevelDetails::CumulativeData,
                        "Chance to hit at least the stat"
                    );
                },
                ChartKind::InterLevelDist => {
                    ui.radio_value(
                        &mut self.reduction_option,
                        ReductionKind::AverageReduction,
                        "Average Stat"
                    );
                    ui.radio_value(
                        &mut self.reduction_option,
                        ReductionKind::BenchmarkReduction,
                        "% to hit Benchmark"
                    );
                },
                _ => {}
            };
        });
        if !matches!(
            (&self.reduction_option, &self.chart_type),
            (&ReductionKind::AverageReduction, &ChartKind::InterLevelDist)
        ) {
            ui.horizontal(|ui| {
                egui::containers::ComboBox::from_label("Stat to Display")
                    .selected_text(format!(
                        "{}",
                        self.selected_stat
                            .entry(self.game_option)
                            .or_insert_with(
                                || look_up_iteration_order(self.game_option)[0].to_string()
                            )
                    ))
                    .show_ui(ui, |ui| {
                        let value =
                            self.selected_stat
                                .entry(self.game_option)
                                .or_insert_with(|| {
                                    look_up_iteration_order(self.game_option)[0].to_string()
                                });

                        self.character
                            .entry(self.game_option)
                            .or_insert_with(|| generate_default_character(self.game_option))
                            .stats
                            .iter_mut()
                            .sorted_by_cached_key(|(key, _value)| {
                                look_up_iteration_order(self.game_option)
                                    .into_iter()
                                    .position(|s| &s == key)
                                    .unwrap()
                            })
                            .for_each(|(key, _stat)| {
                                ui.selectable_value(value, key.to_string(), key);
                            });
                    });

                match self.chart_type {
                    ChartKind::InterLevelDist
                        if matches!(self.reduction_option, ReductionKind::BenchmarkReduction) =>
                    {
                        ui.add(
                            egui::Slider::new(
                                &mut self.benchmark,
                                0..=self
                                    .character
                                    .get(&self.game_option)
                                    .unwrap()
                                    .stats
                                    .get(self.selected_stat.get(&self.game_option).unwrap())
                                    .unwrap()
                                    .cap
                            )
                            .text("Stat Benchmark to hit")
                        );
                    },
                    ChartKind::BoxPlots => {
                        ui.add(
                            Slider::new(&mut self.box_range, 0..=100)
                                .text("Range of stats to be included in the boxes")
                        );
                        reset_button_with(ui, &mut self.box_range, 50);
                    },
                    ChartKind::IntraLevelDist => {
                        ui.add(
                            Slider::new(&mut self.inspected_level, 1..=actual_data.len())
                                .text("Level to focus on")
                        );
                    },
                    _ => {}
                }
            });
        }

        match self.chart_type {
            ChartKind::IntraLevelDist
                if matches!(self.intra_level_option, IntraLevelDetails::DensityData) =>
            {
                let data = actual_data[self.inspected_level - 1]
                    .get(self.selected_stat.get(&self.game_option).unwrap())
                    .unwrap()
                    .iter()
                    .map(|(points, prob)| Bar::new(*points as f64, *prob * 100.0))
                    .collect();

                Plot::new("Exact Plot")
                    .legend(Legend::default())
                    .show(ui, |ui| {
                        ui.bar_chart(
                            BarChart::new(data).name("Probability in % to hit the stat exactly")
                        )
                    });
            },
            ChartKind::IntraLevelDist
                if matches!(self.intra_level_option, IntraLevelDetails::CumulativeData) =>
            {
                let data = actual_data[self.inspected_level - 1]
                    .get(self.selected_stat.get(&self.game_option).unwrap())
                    .unwrap()
                    .iter()
                    .rev()
                    .scan(0.0, |acc, (points, prob)| {
                        *acc += *prob;
                        Some((*points, *acc))
                    })
                    .chain(
                        (0..self
                            .character
                            .get(&self.game_option)
                            .unwrap()
                            .stats
                            .get(self.selected_stat.get(&self.game_option).unwrap())
                            .unwrap()
                            .base)
                            .map(|guaranteed| (guaranteed, 1.0))
                    )
                    .map(|(points, prob)| Bar::new(points as f64, prob * 100.0))
                    .collect();

                Plot::new("Cumulative Plot")
                    .legend(Legend::default())
                    .show(ui, |ui| {
                        ui.bar_chart(
                            BarChart::new(data).name("Probability in % to hit at least the stat")
                        )
                    });
            },
            ChartKind::InterLevelDist
                if matches!(self.reduction_option, ReductionKind::AverageReduction) =>
            {
                let data = actual_data
                    .iter()
                    .map(|stats| {
                        stats
                            .iter()
                            .map(|(name, map)| {
                                (
                                    name,
                                    map.iter().fold(0.0, |acc, (points, prob)| {
                                        acc + *points as f64 * *prob
                                    })
                                )
                            })
                            .collect::<BTreeMap<_, _>>()
                    })
                    .collect::<Vec<_>>();
                let data = look_up_iteration_order(self.game_option)
                    .into_iter()
                    .map(|name| {
                        (
                            name,
                            data.iter()
                                .map(|stats| *stats.get(&name.to_string()).unwrap())
                                .enumerate()
                                .map(|(level, average)| PlotPoint::new((level + 1) as f64, average))
                                .collect::<Vec<_>>()
                        )
                    })
                    .collect::<BTreeMap<_, _>>();

                Plot::new("Average Plot")
                    .legend(Legend::default())
                    .show(ui, |ui| {
                        data.into_iter().for_each(|(name, averages)| {
                            ui.line(
                                Line::new(PlotPoints::Owned(averages))
                                    .name(format!("Average {name}"))
                            );
                        })
                    });
            },
            ChartKind::InterLevelDist
                if matches!(self.reduction_option, ReductionKind::BenchmarkReduction) =>
            {
                let data = actual_data
                    .iter()
                    .enumerate()
                    .map(|(level, stats)| {
                        let stat = stats
                            .get(self.selected_stat.get(&self.game_option).unwrap())
                            .unwrap();
                        Bar::new(
                            (level + 1) as f64,
                            stat.iter()
                                .filter(|(points, _prob)| points >= &&self.benchmark)
                                .map(|(_points, prob)| 100.0 * prob)
                                .sum()
                        )
                    })
                    .collect();

                Plot::new("Benchmark Plot")
                    .legend(Legend::default())
                    .show(ui, |ui| {
                        ui.bar_chart(
                            BarChart::new(data).name("Probability in % to hit the benchmark.")
                        )
                    });
            },
            ChartKind::BoxPlots => {
                let (boxes, series) = actual_data
                    .iter()
                    .enumerate()
                    .map(|(level, stats)| {
                        let stat = stats
                            .get(self.selected_stat.get(&self.game_option).unwrap())
                            .unwrap();
                        (
                            BoxElem::new(
                                (level + 1) as f64,
                                BoxSpread::new(
                                    *stat.keys().min().unwrap_or(&1) as f64,
                                    find_percentile(stat, 0.5 - (self.box_range as f64) / 200.0)
                                        .unwrap_or(5.0),
                                    find_percentile(stat, 0.50).unwrap_or(10.0),
                                    find_percentile(stat, 0.5 + (self.box_range as f64) / 200.0)
                                        .unwrap_or(15.0),
                                    *stat.keys().max().unwrap_or(&20) as f64
                                )
                            ),
                            PlotPoint::new(
                                (level + 1) as f64,
                                stat.iter()
                                    .fold(0.0, |acc, (points, prob)| acc + *points as f64 * *prob)
                            )
                        )
                    })
                    .unzip();
                Plot::new("Box Plot")
                    .legend(Legend::default())
                    .show(ui, |ui| {
                        ui.box_plot(BoxPlot::new(boxes).name("Medians, Percentiles & Extremes"));
                        ui.line(Line::new(PlotPoints::Owned(series)).name("Averages"))
                    });
            },
            _ => {}
        }
    }
}

fn find_percentile(stat : &BTreeMap<u8, f64>, percentile : f64) -> Option<f64> {
    stat.iter()
        .scan(0.0, |acc, (points, prob)| {
            *acc += prob;
            Some((*points, *acc))
        })
        .find(|(_points, prob)| prob >= &percentile)
        .map(|(points, _prob)| points as f64)
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

        self.character_builder(ctx);
        Self::character_progression_builder(ctx);
        self.data_plotting_window(ctx);
        Self::data_manager(ctx);
    }
}
