use chrono::{Datelike, Local, NaiveDate, NaiveTime, Timelike};
use eframe::egui;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

#[derive(Clone)]
struct WorkEntry {
    start: String,
    end: String,
}

struct App {
    month_first: NaiveDate,
    selected_date: Option<NaiveDate>,
    global_rate: f64,
    entries: HashMap<NaiveDate, Vec<WorkEntry>>,
    show_popup: bool,
    temp_start: String,
    temp_end: String,
    csv_path: String,
    popup_error: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        let today = Local::now().naive_local().date();
        let first = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        let mut app = Self {
            month_first: first,
            selected_date: None,
            global_rate: 30.0,
            entries: HashMap::new(),
            show_popup: false,
            temp_start: "".into(),
            temp_end: "".into(),
            csv_path: "work_data.csv".into(),
            popup_error: None,
        };
        app.load_csv();
        app
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        // 단축키로 저장
        if (ctx.input(|i| i.modifiers.command) || ctx.input(|i| i.modifiers.ctrl))
            && ctx.input(|i| i.key_pressed(egui::Key::S))
        {
            self.save_csv();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Hourly (global):");
                ui.add(
                    egui::DragValue::new(&mut self.global_rate)
                        .clamp_range(0.0..=1_000_000.0)
                        .suffix(" $"),
                );
                if ui.button("💾 Save (⌘/Ctrl+S)").clicked() {
                    self.save_csv();
                }
            });

            ui.separator();
            self.calendar_ui(ui);
            ui.separator();

            let (month_total, overall_total) = self.compute_totals();
            ui.heading(format!(
                "📅 This Month: ${:.2}    💰 Overall: ${:.2}",
                month_total, overall_total
            ));

            if self.show_popup {
                if let Some(date) = self.selected_date {
                    egui::Window::new(format!("📅 {}", date))
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            ui.label("Add a work entry (HH:MM, 24h)");
                            ui.horizontal(|ui| {
                                ui.label("Start");
                                ui.text_edit_singleline(&mut self.temp_start);
                                ui.label("End");
                                ui.text_edit_singleline(&mut self.temp_end);
                            });
                            ui.small("Lunch break (30m) is auto-deducted. After 15:30 → 1.5× overtime.");

                            if let Some(err) = &self.popup_error {
                                ui.colored_label(egui::Color32::from_rgb(190, 40, 40), err);
                            }

                            ui.horizontal(|ui| {
                                if ui.button("➕ Save Entry").clicked() {
                                    if let Some(_) = calculate_pay_summary(
                                        &self.temp_start,
                                        &self.temp_end,
                                        self.global_rate,
                                    ) {
                                        self.entries
                                            .entry(date)
                                            .or_default()
                                            .push(WorkEntry {
                                                start: self.temp_start.clone(),
                                                end: self.temp_end.clone(),
                                            });
                                        self.temp_start.clear();
                                        self.temp_end.clear();
                                        self.popup_error = None;
                                    } else {
                                        self.popup_error =
                                            Some("Check time format (HH:MM) and duration.".into());
                                    }
                                }
                                if ui.button("Close").clicked() {
                                    self.show_popup = false;
                                    self.popup_error = None;
                                }
                            });

                            ui.separator();
                            ui.label("Entries on this date:");
                            if let Some(list) = self.entries.get_mut(&date) {
                                let mut remove_idx: Option<usize> = None;
                                for (i, e) in list.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{}.", i + 1));
                                        ui.monospace(format!("{} - {}", e.start, e.end));
                                        if let Some(summary) = calculate_pay_summary(
                                            &e.start,
                                            &e.end,
                                            self.global_rate,
                                        ) {
                                            ui.small(format!(
                                                "{:.2}h reg + {:.2}h OT → ${:.2}",
                                                summary.regular_hours,
                                                summary.overtime_hours,
                                                summary.total_pay
                                            ));
                                        } else {
                                            ui.small("Invalid times");
                                        }
                                        if ui.button("🗑").clicked() {
                                            remove_idx = Some(i);
                                        }
                                    });
                                }
                                if let Some(i) = remove_idx {
                                    list.remove(i);
                                }
                            }
                        });
                }
            }
        });
    }
}

/* ---------- Calendar UI ---------- */

impl App {
    fn calendar_ui(&mut self, ui: &mut egui::Ui) {
        let y = self.month_first.year();
        let m = self.month_first.month();
        let today = Local::now().naive_local().date();
    
        ui.columns(3, |cols| {
            cols[0].with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if ui.button("◀").clicked() {
                    let (ny, nm) = if m == 1 { (y - 1, 12) } else { (y, m - 1) };
                    self.month_first = NaiveDate::from_ymd_opt(ny, nm, 1).unwrap();
                }
            });
            cols[1].vertical_centered(|ui| {
                ui.heading(format!("{} {}", month_name(m), y));
            });
            cols[2].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("▶").clicked() {
                    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
                    self.month_first = NaiveDate::from_ymd_opt(ny, nm, 1).unwrap();
                }
            });
        });
    
        ui.add_space(6.0);
    
        // 🗓 요일 헤더
        let weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        egui::Grid::new("header_grid")
            .num_columns(7)
            .min_col_width(120.0)
            .show(ui, |ui| {
                for w in weekdays {
                    let is_weekend = w == "Sun" || w == "Sat";
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(245, 245, 245))
                        .rounding(egui::Rounding::same(6))
                        .show(ui, |ui| {
                            ui.centered_and_justified(|ui| {
                                ui.label(
                                    egui::RichText::new(w)
                                        .strong()
                                        .color(if is_weekend {
                                            egui::Color32::from_rgb(200, 60, 60)
                                        } else {
                                            egui::Color32::BLACK
                                        }),
                                );
                            });
                        });
                }
                ui.end_row();
            });
    
        let start_wd = self.month_first.weekday().num_days_from_sunday() as usize;
        let days_in_month = last_day(self.month_first.year(), self.month_first.month());
        let mut day: u32 = 1;
        let mut started = false;
        let cell_size = egui::vec2(120.0, 96.0);
        let cell_rounding = egui::Rounding::same(8);
    
        // 📅 달력 테이블
        egui::Grid::new("calendar_grid")
            .num_columns(7)
            .min_col_width(cell_size.x)
            .min_row_height(cell_size.y)
            .show(ui, |ui| {
                for _week in 0..6 {
                    for wd in 0..7usize {
                        if !started && wd == start_wd {
                            started = true;
                        }
    
                        if started && day <= days_in_month {
                            if let Some(date) = NaiveDate::from_ymd_opt(y, m, day) {
                                let is_today = date == today;
                                let is_selected = self.selected_date == Some(date);
                                let is_weekend = wd == 0 || wd == 6; // ✅ 일요일(0) or 토요일(6)
    
                                // 기본 배경색
                                let mut bg = if is_weekend {
                                    egui::Color32::from_rgb(250, 240, 240) // 주말 연한 붉은색
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
    
                                // 선택 및 오늘 강조
                                if is_selected {
                                    bg = egui::Color32::from_rgb(180, 220, 255);
                                } else if is_today {
                                    bg = egui::Color32::from_rgb(230, 240, 250);
                                }
    
                                let border = if is_selected {
                                    egui::Stroke::new(1.5, egui::Color32::from_rgb(50, 120, 200))
                                } else {
                                    egui::Stroke::new(0.5, egui::Color32::from_gray(180))
                                };

                                let resp = ui
                                    .allocate_ui_with_layout(
                                        cell_size,
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            egui::Frame::none()
                                                .fill(bg)
                                                .stroke(border)
                                                .rounding(cell_rounding)
                                                .inner_margin(egui::Margin::same(8))
                                                .show(ui, |ui| {
                                                    ui.set_min_size(cell_size);
                                                    ui.vertical(|ui| {
                                                        ui.horizontal(|ui| {
                                                            ui.strong(day.to_string());
                                                            if is_today && !is_selected {
                                                                ui.add_space(4.0);
                                                                ui.label(
                                                                    egui::RichText::new("Today")
                                                                        .small()
                                                                        .color(
                                                                            egui::Color32::from_rgb(
                                                                                70, 120, 200,
                                                                            ),
                                                                        ),
                                                                );
                                                            }
                                                            ui.with_layout(
                                                                egui::Layout::right_to_left(
                                                                    egui::Align::Center,
                                                                ),
                                                                |ui| {
                                                                    if ui.small_button("+").clicked() {
                                                                        self.selected_date = Some(date);
                                                                        self.show_popup = true;
                                                                    }
                                                                },
                                                            );
                                                        });
                                                        ui.add_space(4.0);

                                                        if let Some(list) = self.entries.get(&date) {
                                                            let mut day_total = 0.0;
                                                            let mut day_hours = 0.0;
                                                            for entry in list.iter() {
                                                                if let Some(summary) =
                                                                    calculate_pay_summary(
                                                                        &entry.start,
                                                                        &entry.end,
                                                                        self.global_rate,
                                                                    )
                                                                {
                                                                    day_total += summary.total_pay;
                                                                    day_hours +=
                                                                        summary.total_hours();
                                                                }
                                                            }
                                                            ui.small(format!(
                                                                "{:.2}h / ${:.2}",
                                                                day_hours, day_total
                                                            ));
                                                            ui.add_space(4.0);
                                                            for entry in list.iter().take(3) {
                                                                ui.small(format!(
                                                                    "{}-{}",
                                                                    entry.start, entry.end
                                                                ));
                                                            }
                                                            if list.len() > 3 {
                                                                ui.small(format!(
                                                                    "+{} more…",
                                                                    list.len() - 3
                                                                ));
                                                            }
                                                        } else {
                                                            ui.add_space(40.0);
                                                        }
                                                    });
                                                });
                                        },
                                    )
                                    .response;
    
                                if resp.clicked() {
                                    self.selected_date = Some(date);
                                    self.show_popup = true;
                                }
                            }
                            day += 1;
                        } else {
                            egui::Frame::none()
                                .stroke(egui::Stroke::new(0.5, egui::Color32::LIGHT_GRAY))
                                .rounding(cell_rounding)
                                .inner_margin(egui::Margin::same(8))
                                .show(ui, |ui| {
                                    ui.add_sized(
                                        [cell_size.x, cell_size.y],
                                        egui::Label::new(""),
                                    );
                                });
                        }
                    }
                    ui.end_row();
                }
            });
    }
    
    fn compute_totals(&self) -> (f64, f64) {
        let y = self.month_first.year();
        let m = self.month_first.month();
        let mut month_total = 0.0;
        let mut all_total = 0.0;

        for (date, list) in &self.entries {
            let mut day_sum = 0.0;
            for e in list {
                if let Some(summary) =
                    calculate_pay_summary(&e.start, &e.end, self.global_rate)
                {
                    day_sum += summary.total_pay;
                }
            }
            all_total += day_sum;
            if date.year() == y && date.month() == m {
                month_total += day_sum;
            }
        }
        (month_total, all_total)
    }
}

/* ---------- CSV I/O ---------- */

impl App {
    fn save_csv(&self) {
        if let Ok(mut f) = File::create(&self.csv_path) {
            let _ = writeln!(
                f,
                "date,start,end,base_rate,regular_hours,overtime_hours,total"
            );
            for (date, list) in &self.entries {
                for e in list {
                    if let Some(summary) =
                        calculate_pay_summary(&e.start, &e.end, self.global_rate)
                    {
                        let _ = writeln!(
                            f,
                            "{},{},{},{:.4},{:.4},{:.4},{:.4}",
                            date,
                            e.start,
                            e.end,
                            self.global_rate,
                            summary.regular_hours,
                            summary.overtime_hours,
                            summary.total_pay
                        );
                    }
                }
            }
            println!("✅ Saved to {}", self.csv_path);
        }
    }

    fn load_csv(&mut self) {
        if let Ok(f) = OpenOptions::new().read(true).open(&self.csv_path) {
            let reader = BufReader::new(f);
            for (i, line) in reader.lines().enumerate() {
                if let Ok(l) = line {
                    if i == 0 && l.to_lowercase().starts_with("date,start,end") {
                        continue;
                    }
                    let parts: Vec<&str> = l.split(',').collect();
                    if parts.len() < 3 {
                        continue;
                    }
                    if let Ok(date) = NaiveDate::parse_from_str(parts[0], "%Y-%m-%d") {
                        let start = parts[1].trim().to_string();
                        let end = parts[2].trim().to_string();
                        self.entries
                            .entry(date)
                            .or_default()
                            .push(WorkEntry { start, end });
                    }
                }
            }
            println!("📂 Loaded from {}", self.csv_path);
        }
    }
}

/* ---------- Utils ---------- */

struct PaySummary {
    regular_hours: f64,
    overtime_hours: f64,
    total_pay: f64,
}

impl PaySummary {
    fn total_hours(&self) -> f64 {
        self.regular_hours + self.overtime_hours
    }
}

fn calculate_pay_summary(start: &str, end: &str, base_rate: f64) -> Option<PaySummary> {
    const MINUTES_PER_DAY: i32 = 24 * 60;
    const OVERTIME_START_MIN: i32 = 15 * 60 + 30; // 15:30
    const LUNCH_BREAK_MIN: i32 = 30;

    let s = parse_hhmm(start)?;
    let e = parse_hhmm(end)?;

    let start_min = (s.num_seconds_from_midnight() / 60) as i32;
    let mut end_min = (e.num_seconds_from_midnight() / 60) as i32;
    if end_min <= start_min {
        end_min += MINUTES_PER_DAY;
    }
    let total_duration = end_min - start_min;
    if total_duration <= 0 {
        return None;
    }

    let mut regular_minutes = 0i32;
    let mut overtime_minutes = 0i32;
    let mut cursor = start_min;

    while cursor < end_min {
        let day_start = (cursor / MINUTES_PER_DAY) * MINUTES_PER_DAY;
        let day_overtime_start = day_start + OVERTIME_START_MIN;
        if cursor < day_overtime_start {
            let segment_end = end_min.min(day_overtime_start);
            regular_minutes += segment_end - cursor;
            cursor = segment_end;
        } else {
            let day_end = day_start + MINUTES_PER_DAY;
            let segment_end = end_min.min(day_end);
            overtime_minutes += segment_end - cursor;
            cursor = segment_end;
        }
    }

    let mut remaining_lunch = LUNCH_BREAK_MIN.min(total_duration);
    if regular_minutes >= remaining_lunch {
        regular_minutes -= remaining_lunch;
        remaining_lunch = 0;
    } else {
        remaining_lunch -= regular_minutes;
        regular_minutes = 0;
    }
    if remaining_lunch > 0 {
        overtime_minutes = (overtime_minutes - remaining_lunch).max(0);
    }

    let worked_minutes = regular_minutes + overtime_minutes;
    if worked_minutes <= 0 {
        return None;
    }

    let regular_hours = regular_minutes as f64 / 60.0;
    let overtime_hours = overtime_minutes as f64 / 60.0;
    let total_pay = regular_hours * base_rate + overtime_hours * base_rate * 1.5;

    Some(PaySummary {
        regular_hours,
        overtime_hours,
        total_pay,
    })
}

fn parse_hhmm(s: &str) -> Option<NaiveTime> {
    let (h, m) = s.split_once(':')?;
    let hh: u32 = h.parse().ok()?;
    let mm: u32 = m.parse().ok()?;
    NaiveTime::from_hms_opt(hh, mm, 0)
}

fn last_day(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };
    next.pred_opt().unwrap().day()
}

fn month_name(m: u32) -> &'static str {
    [
        "",
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ][m as usize]
}

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([760.0, 660.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Money Calendar",
        opts,
        Box::new(|_| Ok(Box::new(App::default()))),
    )
}
