use chrono::{Datelike, Local, NaiveDate, NaiveTime};
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
    calendar_month: NaiveDate,
    selected_date: Option<NaiveDate>,
    hourly_rate: f64,
    entries: HashMap<NaiveDate, Vec<WorkEntry>>,
    temp_start: String,
    temp_end: String,
    csv_path: String,
}

impl Default for App {
    fn default() -> Self {
        let today = Local::now().naive_local().date();
        let first_day = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        let mut app = Self {
            calendar_month: first_day,
            selected_date: None,
            hourly_rate: 30.0,
            entries: HashMap::new(),
            temp_start: "".into(),
            temp_end: "".into(),
            csv_path: "work_data.csv".into(),
        };
        app.load_from_csv(); // ✅ 실행 시 자동 불러오기
        app
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        // ✅ 단축키 감지 (Command+S or Ctrl+S)
        if ctx.input(|i| i.modifiers.command || i.modifiers.ctrl)
            && ctx.input(|i| i.key_pressed(egui::Key::S))
        {
            self.save_to_csv();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("💵 Hourly Rate: ");
                ui.add(egui::DragValue::new(&mut self.hourly_rate).suffix(" $"));
                if ui.button("💾 Save").clicked() {
                    self.save_to_csv();
                }
            });

            ui.separator();
            self.draw_calendar(ui);
            ui.separator();

            if let Some(date) = self.selected_date {
                ui.heading(format!("📅 {}", date));
                ui.horizontal(|ui| {
                    ui.label("Start");
                    ui.text_edit_singleline(&mut self.temp_start);
                    ui.label("End");
                    ui.text_edit_singleline(&mut self.temp_end);
                    if ui.button("➕ Add").clicked() {
                        self.entries.entry(date).or_default().push(WorkEntry {
                            start: self.temp_start.clone(),
                            end: self.temp_end.clone(),
                        });
                        self.temp_start.clear();
                        self.temp_end.clear();
                    }
                });

                if let Some(list) = self.entries.get(&date) {
                    for (i, e) in list.iter().enumerate() {
                        ui.label(format!("{}: {} - {}", i + 1, e.start, e.end));
                    }
                }
            }

            ui.separator();
            self.show_total(ui);
            ui.small("💡 Tip: Press Command+S (Mac) or Ctrl+S (Windows/Linux) to save.");
        });
    }
}

impl App {
    // 🗓️ 달력 UI
    fn draw_calendar(&mut self, ui: &mut egui::Ui) {
        let month = self.calendar_month.month();
        let year = self.calendar_month.year();
        let today = Local::now().naive_local().date();

        ui.horizontal(|ui| {
            if ui.button("◀").clicked() {
                self.calendar_month = if month == 1 {
                    NaiveDate::from_ymd_opt(year - 1, 12, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(year, month - 1, 1).unwrap()
                };
            }

            ui.heading(format!("{} {}", month_name(month), year));

            if ui.button("▶").clicked() {
                self.calendar_month = if month == 12 {
                    NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
                };
            }
        });

        ui.separator();
        let weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        ui.horizontal(|ui| {
            for w in weekdays {
                ui.label(egui::RichText::new(w).strong());
            }
        });

        let first_weekday = self.calendar_month.weekday().num_days_from_sunday();
        let days_in_month = last_day_of_month(year, month);
        let mut day = 1u32;
        let mut started = false;

        for _ in 0..6 {
            let mut row_empty = true;
            ui.horizontal(|ui| {
                for wd in 0..7 {
                    if !started && wd == first_weekday {
                        started = true;
                    }

                    if started && day <= days_in_month {
                        let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
                        let is_today = date == today;
                        let selected = self.selected_date == Some(date);
                        let has_entry = self.entries.contains_key(&date);

                        let mut text = format!("{}", day);
                        if has_entry {
                            text = format!("{}📝", day);
                        }

                        let mut button = egui::Button::new(text);
                        if is_today {
                            button = button.fill(egui::Color32::from_rgb(230, 240, 250));
                        }
                        if selected {
                            button = button.fill(egui::Color32::from_rgb(180, 220, 255));
                        }

                        if ui.add(button).clicked() {
                            self.selected_date = Some(date);
                        }

                        day += 1;
                        row_empty = false;
                    } else {
                        ui.label(" ");
                    }
                }
            });
            if row_empty {
                break;
            }
        }
    }

    // 💾 CSV 저장
    fn save_to_csv(&self) {
        if let Ok(mut file) = File::create(&self.csv_path) {
            for (date, entries) in &self.entries {
                for e in entries {
                    let line = format!("{},{},{}\n", date, e.start, e.end);
                    let _ = file.write_all(line.as_bytes());
                }
            }
            println!("✅ Data saved to {}", self.csv_path);
        }
    }

    // 📥 CSV 로드
    fn load_from_csv(&mut self) {
        let path = &self.csv_path;
        let file = OpenOptions::new().read(true).open(path);
        if let Ok(f) = file {
            let reader = BufReader::new(f);
            for line in reader.lines() {
                if let Ok(l) = line {
                    let parts: Vec<&str> = l.split(',').collect();
                    if parts.len() == 3 {
                        if let Ok(date) = NaiveDate::parse_from_str(parts[0], "%Y-%m-%d") {
                            self.entries.entry(date).or_default().push(WorkEntry {
                                start: parts[1].to_string(),
                                end: parts[2].to_string(),
                            });
                        }
                    }
                }
            }
            println!("📂 Data loaded from {}", self.csv_path);
        }
    }

    fn show_total(&self, ui: &mut egui::Ui) {
        let mut total_hours = 0.0;
        for (_, entries) in &self.entries {
            for e in entries {
                if let (Some(s), Some(t)) = (parse_hhmm(&e.start), parse_hhmm(&e.end)) {
                    let mut diff = (t - s).num_minutes() as f64 / 60.0;
                    if diff < 0.0 {
                        diff += 24.0;
                    }
                    total_hours += diff;
                }
            }
        }
        let total_wage = total_hours * self.hourly_rate;
        ui.heading(format!(
            "🕒 Total Hours: {:.2}   |   💵 Total Wage: ${:.2}",
            total_hours, total_wage
        ));
    }
}

// 🔸 유틸 함수
fn last_day_of_month(year: i32, month: u32) -> u32 {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };
    (next_month.pred_opt().unwrap()).day()
}

fn month_name(m: u32) -> &'static str {
    match m {
        1 => "January", 2 => "February", 3 => "March", 4 => "April", 5 => "May", 6 => "June",
        7 => "July", 8 => "August", 9 => "September", 10 => "October", 11 => "November", 12 => "December",
        _ => "",
    }
}

fn parse_hhmm(s: &str) -> Option<NaiveTime> {
    let (h, m) = s.split_once(':')?;
    let hh: u32 = h.parse().ok()?;
    let mm: u32 = m.parse().ok()?;
    NaiveTime::from_hms_opt(hh, mm, 0)
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native("📅 Work Calendar (CSV Auto Save)", options, Box::new(|_cc| Ok(Box::new(App::default()))))
}
