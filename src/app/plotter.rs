use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    ops::Deref
};

use super::{
    progression::{ConcreteStatChange, UsefulStatChange},
    sit::StatIndexType,
    CompleteData, GameData, UsefulId
};
use cached::proc_macro::cached;
use egui::{
    plot::{
        uniform_grid_spacer, Bar, BarChart, BoxElem, BoxPlot, BoxSpread, GridMark, Legend, Line,
        Plot, PlotPoint, PlotPoints
    },
    reset_button_with, Align, Id, Layout, Slider, Ui
};
use fe_levels::{Character, StatType};
use itertools::Itertools;
use poll_promise::Promise;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Default, Deserialize, Serialize)]
enum ChartKind {
    IntraLevelDist,
    InterLevelDist,
    #[default]
    BoxPlots
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

#[derive(Deserialize, Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct PlotterData {
    chart_type : ChartKind,
    benchmark : StatType,
    box_range : u8,
    inspected_level : usize,
    selected_stat : StatIndexType,
    intra_level_option : IntraLevelDetails,
    reduction_option : ReductionKind,
    window_id : UsefulId
}

impl Default for PlotterData {
    fn default() -> Self {
        Self {
            chart_type : Default::default(),
            benchmark : Default::default(),
            box_range : 50,
            inspected_level : Default::default(),
            selected_stat : StatIndexType::arbitrary_valid(Default::default()),
            intra_level_option : Default::default(),
            reduction_option : Default::default(),
            window_id : Default::default()
        }
    }
}

impl PlotterData {
    pub fn id(&self) -> Id { Id::new(self.window_id) }
}

#[derive(Deserialize, Serialize, Default)]
pub struct PlotterManager {
    #[serde(skip)]
    derived_data : Option<
        Promise<(
            Vec<ConcreteStatChange>,
            Character<StatIndexType>,
            CompleteData
        )>
    >,
    plotter_windows : Vec<PlotterData>
}

pub fn actual_data_display(
    context : &GameData,
    data : &mut PlotterData,
    ui : &mut Ui,
    actual_data : &CompleteData,
    new_window : &mut Option<PlotterData>
) {
    if let Some(first) = actual_data.first() {
        if first.get(&data.selected_stat).is_none() {
            data.selected_stat = *first.iter().next().unwrap().0;
        }
    }
    data.inspected_level = data.inspected_level.clamp(1, actual_data.len());

    ui.horizontal_top(|ui| {
        egui::containers::ComboBox::from_label("Data to Display")
            .selected_text(data.chart_type.to_string())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut data.chart_type,
                    ChartKind::IntraLevelDist,
                    ChartKind::IntraLevelDist.to_string()
                );
                ui.selectable_value(
                    &mut data.chart_type,
                    ChartKind::InterLevelDist,
                    ChartKind::InterLevelDist.to_string()
                );
                ui.selectable_value(
                    &mut data.chart_type,
                    ChartKind::BoxPlots,
                    ChartKind::BoxPlots.to_string()
                );
            });
        match data.chart_type {
            ChartKind::IntraLevelDist => {
                ui.radio_value(
                    &mut data.intra_level_option,
                    IntraLevelDetails::DensityData,
                    "Chance to hit the stat exactly"
                );
                ui.radio_value(
                    &mut data.intra_level_option,
                    IntraLevelDetails::CumulativeData,
                    "Chance to hit at least the stat"
                );
            },
            ChartKind::InterLevelDist => {
                ui.radio_value(
                    &mut data.reduction_option,
                    ReductionKind::AverageReduction,
                    "Average Stat"
                );
                ui.radio_value(
                    &mut data.reduction_option,
                    ReductionKind::BenchmarkReduction,
                    "% to hit Benchmark"
                );
            },
            _ => {}
        };
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            if ui.button("Add Plotter").clicked() {
                *new_window = Some(Default::default());
            }
        });
    });
    if !matches!(
        (&data.reduction_option, &data.chart_type),
        (&ReductionKind::AverageReduction, &ChartKind::InterLevelDist)
    ) {
        ui.horizontal(|ui| {
            egui::containers::ComboBox::from_label("Stat to Display")
                .selected_text(format!("{}", data.selected_stat))
                .show_ui(ui, |ui| {
                    context
                        .character
                        .stats
                        .iter()
                        .sorted_by_key(|(key, _value)| **key)
                        .for_each(|(key, _stat)| {
                            ui.selectable_value(&mut data.selected_stat, *key, key.to_string());
                        });
                });

            match data.chart_type {
                ChartKind::InterLevelDist
                    if matches!(data.reduction_option, ReductionKind::BenchmarkReduction) =>
                {
                    ui.add(
                        egui::Slider::new(
                            &mut data.benchmark,
                            0..=actual_data
                                .last()
                                .unwrap()
                                .get(&data.selected_stat)
                                .unwrap()
                                .iter()
                                .map(|(stat, _prob)| *stat)
                                .max()
                                .unwrap()
                        )
                        .text("Stat Benchmark to hit")
                    );
                },
                ChartKind::BoxPlots => {
                    ui.add(
                        Slider::new(&mut data.box_range, 0..=100)
                            .text("Range of stats to be included in the boxes")
                    );
                    reset_button_with(ui, &mut data.box_range, 50);
                },
                ChartKind::IntraLevelDist => {
                    ui.add(
                        Slider::new(&mut data.inspected_level, 1..=actual_data.len())
                            .text("Level to focus on")
                    );
                },
                _ => {}
            }
        });
    }

    match data.chart_type {
        ChartKind::IntraLevelDist
            if matches!(data.intra_level_option, IntraLevelDetails::DensityData) =>
        {
            let selected_data_range = &actual_data[data.inspected_level - 1]
                .get(&data.selected_stat)
                .unwrap();
            let bars = selected_data_range
                .iter()
                .map(|(points, prob)| Bar::new(*points as f64, *prob * 100.0))
                .collect();
            let max = selected_data_range
                .iter()
                .map(|(value, _p)| value)
                .max()
                .unwrap();

            Plot::new("Exact Plot")
                .legend(Legend::default())
                .include_x(-0.2)
                .include_x(*max as f64 + 0.5)
                .include_y(-0.5)
                .include_y(110.0)
                .show(ui, |ui| {
                    ui.bar_chart(
                        BarChart::new(bars).name("Probability in % to hit the stat exactly")
                    );
                });
        },
        ChartKind::IntraLevelDist
            if matches!(data.intra_level_option, IntraLevelDetails::CumulativeData) =>
        {
            let selected_data_range = &actual_data[data.inspected_level - 1]
                .get(&data.selected_stat)
                .unwrap();
            let data = selected_data_range
                .iter()
                .rev()
                .scan(0.0, |acc, (points, prob)| {
                    *acc += *prob;
                    Some((*points, *acc))
                })
                .chain(
                    (0..*selected_data_range
                        .iter()
                        .map(|(stat, _prob)| stat)
                        .min()
                        .unwrap())
                        .map(|guaranteed| (guaranteed, 1.0))
                )
                .map(|(points, prob)| Bar::new(points as f64, prob * 100.0))
                .collect();
            let max = selected_data_range
                .iter()
                .map(|(value, _p)| value)
                .max()
                .unwrap();

            Plot::new("Cumulative Plot")
                .legend(Legend::default())
                .include_x(-0.2)
                .include_x(*max as f64 + 0.5)
                .include_y(-0.5)
                .include_y(110.0)
                .show(ui, |ui| {
                    ui.bar_chart(
                        BarChart::new(data).name("Probability in % to hit at least the stat")
                    )
                });
        },
        ChartKind::InterLevelDist
            if matches!(data.reduction_option, ReductionKind::AverageReduction) =>
        {
            let data = actual_data
                .iter()
                .map(|stats| {
                    stats
                        .iter()
                        .map(|(name, map)| {
                            (
                                name,
                                map.iter()
                                    .fold(0.0, |acc, (points, prob)| acc + *points as f64 * *prob)
                            )
                        })
                        .collect::<BTreeMap<_, _>>()
                })
                .collect::<Vec<_>>();
            let data = StatIndexType::new(context.game_option)
                .into_iter()
                .map(|stat_type| {
                    (
                        stat_type,
                        data.iter()
                            .map(|stats| *stats.get(&stat_type).unwrap())
                            .enumerate()
                            .map(|(level, average)| PlotPoint::new((level + 1) as f64, average))
                            .collect::<Vec<_>>()
                    )
                })
                .collect::<BTreeMap<_, _>>();

            let max = &actual_data
                .last()
                .unwrap()
                .iter()
                .map(|(_sit, tree)| tree.keys().max().unwrap())
                .max()
                .unwrap();

            let copied_progression = context.progression.clone();
            let copied_name = context.character.name.clone();
            let important_marks : BTreeSet<_> = context
                .progression
                .iter()
                .map(UsefulStatChange::marking_worthy)
                .enumerate()
                .filter(|(_index, val)| *val)
                .map(|(index, _truthy)| index + 2)
                .chain(std::iter::once(1))
                .chain(std::iter::once(context.progression.len() + 1))
                .collect();

            Plot::new("Average Plot")
                .legend(Legend::default())
                .include_x(-0.2)
                .include_x(actual_data.len() as f64 + 0.5)
                .include_y(-0.5)
                .include_y(**max as f64 * 1.2)
                .label_formatter(|name, point| {
                    if !name.is_empty() {
                        format!("{name}: {:.1}", point.y)
                    }
                    else {
                        "".to_owned()
                    }
                })
                .x_axis_formatter(move |value, _visible_range| {
                    if value == 1.0 {
                        format!("Base {}", copied_name)
                    }
                    else if value >= 2.0 {
                        copied_progression
                            .get((value - 2.0) as usize)
                            .map(|sc| format!("after {sc}"))
                            .unwrap_or_else(|| "".to_owned())
                    }
                    else {
                        "".to_owned()
                    }
                })
                .x_grid_spacer(move |grid_input| {
                    let (lower, upper) = grid_input.bounds;
                    let mut current = lower.round();
                    std::iter::from_fn(|| {
                        let out = current;
                        current += 1.0;
                        (out <= upper).then_some(out)
                    })
                    .filter(|x| x >= &lower)
                    .map(|mark| GridMark {
                        value : mark,
                        step_size : if important_marks.contains(&(mark as usize)) {
                            10.0
                        }
                        else {
                            1.0
                        }
                    })
                    .collect()
                })
                .y_grid_spacer(uniform_grid_spacer(|_grid_input| [10.0, 1.0, 0.1]))
                .show(ui, |ui| {
                    data.into_iter().for_each(|(name, averages)| {
                        ui.line(
                            Line::new(PlotPoints::Owned(averages)).name(format!("Average {name}"))
                        );
                    })
                });
        },
        ChartKind::InterLevelDist
            if matches!(data.reduction_option, ReductionKind::BenchmarkReduction) =>
        {
            let data = actual_data
                .iter()
                .enumerate()
                .map(|(level, stats)| {
                    let stat = stats.get(&data.selected_stat).unwrap();
                    Bar::new(
                        (level + 1) as f64,
                        stat.iter()
                            .filter(|(points, _prob)| points >= &&data.benchmark)
                            .map(|(_points, prob)| 100.0 * prob)
                            .sum()
                    )
                })
                .collect();

            Plot::new("Benchmark Plot")
                .legend(Legend::default())
                .include_x(-0.2)
                .include_x(actual_data.len() as f64 + 0.5)
                .include_y(-0.5)
                .include_y(110.0)
                .show(ui, |ui| {
                    ui.bar_chart(BarChart::new(data).name("Probability in % to hit the benchmark."))
                });
        },
        ChartKind::BoxPlots => {
            let (boxes, series) = actual_data
                .iter()
                .enumerate()
                .map(|(level, stats)| {
                    let stat = stats.get(&data.selected_stat).unwrap();
                    (
                        BoxElem::new(
                            (level + 1) as f64,
                            BoxSpread::new(
                                *stat.keys().min().unwrap_or(&1) as f64,
                                find_percentile(stat, 0.5 - (data.box_range as f64) / 200.0)
                                    .unwrap_or(5.0),
                                find_percentile(stat, 0.50).unwrap_or(10.0),
                                find_percentile(stat, 0.5 + (data.box_range as f64) / 200.0)
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
            let max = &actual_data
                .last()
                .unwrap()
                .iter()
                .map(|(_sit, tree)| tree.keys().max().unwrap())
                .max()
                .unwrap();
            Plot::new("Box Plot")
                .legend(Legend::default())
                .include_x(-0.2)
                .include_x(actual_data.len() as f64 + 0.5)
                .include_y(-0.5)
                .include_y(**max as f64 * 1.2)
                .show(ui, |ui| {
                    ui.box_plot(BoxPlot::new(boxes).name("Medians, Percentiles & Extremes"));
                    ui.line(Line::new(PlotPoints::Owned(series)).name("Averages"))
                });
        },
        _ => {}
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

pub fn data_plotting_windows(context : &mut GameData, ctx : &egui::Context) {
    let copy = std::mem::take(&mut context.plotter.derived_data);

    if let Some(promise) = copy {
        match promise.ready() {
            None => {
                egui::Window::new("Data Plotter").show(ctx, |ui| {
                    ui.spinner();
                    ui.label("Processing...");
                });
                context.plotter.derived_data = Some(promise);
            },
            Some((parameters, character, actual_data))
                if parameters == context.progression.deref() && character == &context.character =>
            {
                if context.plotter.plotter_windows.is_empty() {
                    context.plotter.plotter_windows.push(Default::default());
                }
                let moved_out = std::mem::take(&mut context.plotter.plotter_windows);
                context.plotter.plotter_windows = moved_out
                    .into_iter()
                    .flat_map(|mut state| {
                        let mut currently_open = true;
                        let mut new_instance = None;
                        egui::Window::new("Data Plotter")
                            .id(state.id())
                            .open(&mut currently_open)
                            .show(ctx, |ui| {
                                actual_data_display(
                                    context,
                                    &mut state,
                                    ui,
                                    actual_data,
                                    &mut new_instance
                                );
                            });
                        vec![currently_open.then_some(state), new_instance]
                    })
                    .flatten()
                    .collect();

                context.plotter.derived_data = Some(promise);
            },
            Some((parameters, character, _actual_data))
                if parameters != context.progression.deref() || character != &context.character =>
            {
                egui::Window::new("Data Plotter").show(ctx, |ui| {
                    ui.spinner();
                    ui.label("Processing...");
                });
                context.plotter.derived_data = None;
            },
            _ => unreachable!()
        }
    }
    if context.plotter.derived_data.is_none() {
        if context
            .progression
            .iter()
            .all(ConcreteStatChange::cheap_to_execute)
        {
            let (sender, promise) = Promise::new();
            let character = context.character.clone();
            let progression = context.progression.clone();
            sender.send((
                progression.clone(),
                character.clone(),
                compute(character, progression, None)
            ));
            context.plotter.derived_data = Some(promise);
        }
        else {
            #[cfg(target_arch = "wasm32")]
            {
                egui::Window::new("Error")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(format!(
                            "Unfortunately, operation in a browser environment is slow and \
                             time-constrained. Therefore certain slow stat changing progressions \
                             cannot reasonably be computed. Please remove the following listed \
                             progressions entries or use the native version of this app."
                        ));
                        context
                            .progression
                            .iter()
                            .filter(|sc| sc.cheap_to_execute())
                            .for_each(|sc| {
                                ui.label(sc.to_string());
                            });
                    });
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let character = context.character.clone();
                let progression = context.progression.clone();
                context.plotter.derived_data = Some(Promise::spawn_thread(
                    "Background Compute Thread",
                    move || {
                        (
                            progression.clone(),
                            character.clone(),
                            compute(character, progression, Some(1u64 << 20))
                        )
                    }
                ));
            }
        }
    }
}

#[cached(size = 1000)]
fn compute(
    character : Character<StatIndexType>,
    stat_changes : Vec<ConcreteStatChange>,
    num_samples : Option<u64>
) -> CompleteData {
    fe_levels::generate_histograms(
        &stat_changes
            .into_iter()
            .map(ConcreteStatChange::compile)
            .collect_vec(),
        &character,
        num_samples
    )
}
