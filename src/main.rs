#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Instant;

// ---------- YAML loading ----------

#[derive(Debug, Deserialize)]
struct SegmentSpec {
    name: String,
    duration: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SegmentsFile {
    segments: Vec<SegmentSpec>,
}

struct Segment {
    name: String,
    description: Option<String>,
    duration_secs: i64,
}

fn parse_duration(raw: &str) -> Result<i64, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("empty duration".into());
    }

    if s.contains(':') {
        let parts: Vec<&str> = s.split(':').collect();
        let nums: Result<Vec<i64>, _> = parts.iter().map(|p| p.trim().parse::<i64>()).collect();
        let nums = nums.map_err(|_| format!("invalid duration '{}'", raw))?;
        return match nums.len() {
            2 => Ok(nums[0] * 60 + nums[1]),
            3 => Ok(nums[0] * 3600 + nums[1] * 60 + nums[2]),
            _ => Err(format!("invalid colon duration '{}'", raw)),
        };
    }

    let mut total: i64 = 0;
    let mut acc = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            acc.push(c);
        } else if c.is_whitespace() {
            continue;
        } else {
            let n: i64 = acc
                .parse()
                .map_err(|_| format!("invalid duration '{}'", raw))?;
            acc.clear();
            let mul = match c.to_ascii_lowercase() {
                's' => 1,
                'm' => 60,
                'h' => 3600,
                _ => return Err(format!("unknown unit '{}' in '{}'", c, raw)),
            };
            total += n * mul;
        }
    }
    if !acc.is_empty() {
        let n: i64 = acc
            .parse()
            .map_err(|_| format!("invalid duration '{}'", raw))?;
        total += n;
    }
    if total <= 0 {
        return Err(format!("duration must be positive: '{}'", raw));
    }
    Ok(total)
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("segments.yaml"));
        out.push(cwd.join("segments.yml"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join("segments.yaml"));
            out.push(dir.join("segments.yml"));
        }
    }
    out
}

fn load_segments(explicit: Option<&PathBuf>) -> Result<(PathBuf, Vec<Segment>), String> {
    let (path, contents) = match explicit {
        Some(p) => {
            let s = std::fs::read_to_string(p)
                .map_err(|e| format!("cannot read {}: {}", p.display(), e))?;
            (p.clone(), s)
        }
        None => {
            let candidates = candidate_paths();
            candidates
                .iter()
                .find_map(|p| std::fs::read_to_string(p).ok().map(|s| (p.clone(), s)))
                .ok_or_else(|| {
                    let list = candidates
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n  ");
                    format!("segments.yaml not found in:\n  {}", list)
                })?
        }
    };
    let parsed: SegmentsFile =
        serde_yaml::from_str(&contents).map_err(|e| format!("yaml parse error: {}", e))?;
    let segs = parsed
        .segments
        .into_iter()
        .map(|s| {
            parse_duration(&s.duration)
                .map(|d| Segment {
                    name: s.name.clone(),
                    description: s.description.clone(),
                    duration_secs: d,
                })
                .map_err(|e| format!("segment '{}': {}", s.name, e))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((path, segs))
}

// ---------- App ----------

struct App {
    segments: Vec<Segment>,
    config_path: Option<PathBuf>,
    idx: usize,
    remaining: f64,
    paused: bool,
    last_tick: Option<Instant>,
    error: Option<String>,
    agenda_open: bool,
    screenshot_path: Option<PathBuf>,
    screenshot_frame: u32,
}

impl App {
    fn new(screenshot_path: Option<PathBuf>) -> Self {
        let mut app = Self {
            segments: Vec::new(),
            config_path: None,
            idx: 0,
            remaining: 0.0,
            paused: true,
            last_tick: None,
            error: None,
            agenda_open: false,
            screenshot_path,
            screenshot_frame: 0,
        };
        app.reload();
        app
    }

    fn reload(&mut self) {
        let explicit = self.config_path.clone();
        self.load_from(explicit.as_ref());
    }

    fn load_from(&mut self, explicit: Option<&PathBuf>) {
        match load_segments(explicit) {
            Ok((path, segs)) => {
                self.config_path = Some(path);
                self.segments = segs;
                self.idx = 0;
                self.remaining = self
                    .segments
                    .first()
                    .map(|s| s.duration_secs as f64)
                    .unwrap_or(0.0);
                self.paused = true;
                self.last_tick = None;
                self.error = None;
            }
            Err(e) => {
                self.segments.clear();
                self.error = Some(e);
            }
        }
    }

    fn pick_file(&mut self) {
        let start_dir = self
            .config_path
            .as_ref()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .or_else(|| std::env::current_dir().ok());
        let mut dlg = rfd::FileDialog::new()
            .add_filter("YAML", &["yaml", "yml"])
            .set_title("Load workshop segments");
        if let Some(d) = start_dir {
            dlg = dlg.set_directory(d);
        }
        if let Some(path) = dlg.pick_file() {
            self.load_from(Some(&path));
        }
    }

    fn goto(&mut self, target: i64) {
        if self.segments.is_empty() {
            return;
        }
        let n = self.segments.len() as i64;
        let i = target.clamp(0, n - 1) as usize;
        self.idx = i;
        self.remaining = self.segments[i].duration_secs as f64;
        self.paused = false; // auto-play on jump
        self.last_tick = None;
    }

    fn toggle_pause(&mut self) {
        if self.segments.is_empty() {
            return;
        }
        self.paused = !self.paused;
        self.last_tick = None;
    }
}

fn format_time(secs: f64) -> String {
    let neg = secs < 0.0;
    let total = secs.abs().floor() as i64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    let core = if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    };
    if neg {
        format!("-{}", core)
    } else {
        core
    }
}

// ---------- UI helpers ----------

fn icon_button(ui: &mut egui::Ui, glyph: &str, tip: &str, size: f32) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::click());

    let bg = if response.is_pointer_button_down_on() {
        egui::Color32::from_white_alpha(60)
    } else if response.hovered() {
        egui::Color32::from_white_alpha(35)
    } else {
        egui::Color32::from_white_alpha(10)
    };

    let color = egui::Color32::from_rgb(220, 220, 225);
    let painter = ui.painter();
    painter.rect_filled(rect, egui::Rounding::same(6.0), bg);

    // Center on the actual ink rectangle (mesh_bounds), not the line-height
    // galley rect. That eliminates baseline/descender offsets and lines up
    // glyphs of different shapes within the same button bbox.
    let galley = painter.layout_no_wrap(
        glyph.to_string(),
        egui::FontId::proportional(size),
        color,
    );
    let top_left = rect.center() - galley.mesh_bounds.center().to_vec2();
    painter.galley(top_left, galley, color);

    response.on_hover_text(tip)
}

fn open_in_default_editor(path: &PathBuf) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // `cmd /C start "" <path>` — the empty string is start's "title" placeholder
        // so paths with spaces aren't misinterpreted as the title.
        std::process::Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

fn save_color_image_png(path: &PathBuf, image: &egui::ColorImage) -> Result<(), String> {
    let w = image.size[0] as u32;
    let h = image.size[1] as u32;
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for c in &image.pixels {
        data.push(c.r());
        data.push(c.g());
        data.push(c.b());
        data.push(c.a());
    }
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(&data)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    #[cfg(target_os = "windows")]
    {
        // Prefer Segoe Fluent Icons (Win11); fall back to Segoe MDL2 Assets (Win10).
        // These are designed-for-buttons icon fonts with consistent square metrics.
        let icon_font = ["C:\\Windows\\Fonts\\SegoeIcons.ttf", "C:\\Windows\\Fonts\\segmdl2.ttf"]
            .iter()
            .find(|p| std::path::Path::new(p).exists())
            .copied();
        if let Some(path) = icon_font {
            if let Ok(bytes) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("fluent_icons".to_owned(), egui::FontData::from_owned(bytes));
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push("fluent_icons".to_owned());
            }
        }
        // Keep Segoe UI Symbol as a secondary fallback for any other glyphs.
        if let Ok(bytes) = std::fs::read("C:\\Windows\\Fonts\\seguisym.ttf") {
            fonts.font_data.insert(
                "segoe_ui_symbol".to_owned(),
                egui::FontData::from_owned(bytes),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("segoe_ui_symbol".to_owned());
        }
    }
    ctx.set_fonts(fonts);
}

// ---------- Render ----------

impl eframe::App for App {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // tick clock
        let now = Instant::now();
        if !self.paused && !self.segments.is_empty() {
            if let Some(last) = self.last_tick {
                let dt = now.duration_since(last).as_secs_f64();
                self.remaining -= dt;
            }
        }
        self.last_tick = Some(now);

        // frame background — blinks past zero
        let over = !self.segments.is_empty() && self.remaining <= 0.0;
        let bg = if over {
            let t = ctx.input(|i| i.time);
            if (t * 4.0).sin() > 0.0 {
                egui::Color32::from_rgb(220, 38, 38)
            } else {
                egui::Color32::from_rgb(120, 22, 22)
            }
        } else {
            egui::Color32::from_rgb(22, 24, 30)
        };

        let panel_frame = egui::Frame::default()
            .fill(bg)
            .rounding(egui::Rounding::same(8.0))
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_white_alpha(22),
            ))
            .inner_margin(egui::Margin::symmetric(8.0, 4.0));

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                // Whole-bar interaction: drag to move, right-click for menu
                let drag_id = ui.id().with("bar_drag");
                let drag_resp =
                    ui.interact(ui.max_rect(), drag_id, egui::Sense::click_and_drag());
                if drag_resp.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                drag_resp.context_menu(|ui| {
                    let has_config = self.config_path.is_some();
                    let has_segments = !self.segments.is_empty();
                    if ui
                        .add_enabled(has_segments, egui::Button::new("Show agenda\u{2026}"))
                        .clicked()
                    {
                        self.agenda_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(has_config, egui::Button::new("Edit YAML\u{2026}"))
                        .clicked()
                    {
                        if let Some(p) = &self.config_path {
                            let _ = open_in_default_editor(p);
                        }
                        ui.close_menu();
                    }
                    if ui.button("Load YAML\u{2026}").clicked() {
                        self.pick_file();
                        ui.close_menu();
                    }
                    if ui.button("Reload").clicked() {
                        self.reload();
                        ui.close_menu();
                    }
                    ui.separator();
                    if let Some(p) = &self.config_path {
                        ui.label(
                            egui::RichText::new(p.display().to_string())
                                .small()
                                .weak(),
                        );
                    }
                    ui.separator();
                    if ui.button("Close").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close_menu();
                    }
                });

                ui.horizontal_centered(|ui| {
                    // status dot
                    let dot_color = if over {
                        egui::Color32::from_rgb(239, 68, 68)
                    } else if self.paused {
                        egui::Color32::from_rgb(250, 204, 21)
                    } else {
                        egui::Color32::from_rgb(74, 222, 128)
                    };
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.5, dot_color);

                    // name label "4/8 Q&A" — short, hover shows description
                    let name_text = if self.error.is_some() {
                        "load error".to_string()
                    } else if let Some(seg) = self.segments.get(self.idx) {
                        format!("{}/{} {}", self.idx + 1, self.segments.len(), seg.name)
                    } else {
                        "No segments".to_string()
                    };

                    let config_display = self
                        .config_path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "<none>".to_string());
                    let hover_text = if let Some(err) = &self.error {
                        err.clone()
                    } else if let Some(seg) = self.segments.get(self.idx) {
                        let mut parts = Vec::new();
                        if let Some(desc) = &seg.description {
                            parts.push(desc.clone());
                        }
                        parts.push(format!(
                            "Duration: {}",
                            format_time(seg.duration_secs as f64)
                        ));
                        parts.push(format!("Right-click for menu"));
                        parts.push(format!("Config: {}", config_display));
                        parts.join("\n")
                    } else {
                        format!("Right-click for menu\nConfig: {}", config_display)
                    };

                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(name_text)
                                .size(13.0)
                                .color(egui::Color32::from_rgb(235, 235, 240)),
                        )
                        .truncate()
                        .selectable(false),
                    )
                    .on_hover_text(hover_text);

                    // Right-aligned: buttons (RTL) then time
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            // Filled Segoe UI Symbol media glyphs + ASCII close.
                            if icon_button(ui, "\u{00D7}", "Close", 20.0).clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            if icon_button(ui, "\u{21BB}", "Reload segments.yaml", 16.0)
                                .clicked()
                            {
                                self.reload();
                            }
                            if icon_button(ui, "\u{23ED}", "Next", 15.0).clicked() {
                                self.goto(self.idx as i64 + 1);
                            }
                            let pause_label = if self.paused { "\u{25B6}" } else { "\u{23F8}" };
                            let pause_tip = if self.paused { "Play" } else { "Pause" };
                            if icon_button(ui, pause_label, pause_tip, 15.0).clicked() {
                                self.toggle_pause();
                            }
                            if icon_button(ui, "\u{23EE}", "Previous", 15.0).clicked() {
                                self.goto(self.idx as i64 - 1);
                            }

                            ui.add_space(6.0);

                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format_time(self.remaining))
                                        .monospace()
                                        .size(18.0)
                                        .color(egui::Color32::WHITE),
                                )
                                .selectable(false),
                            );
                        },
                    );
                });
            });

        // Screenshot mode: capture after a few frames, save, exit.
        if self.screenshot_path.is_some() {
            self.screenshot_frame += 1;
            if self.screenshot_frame == 20 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
            let captured: Option<std::sync::Arc<egui::ColorImage>> = ctx.input(|i| {
                for ev in &i.raw.events {
                    if let egui::Event::Screenshot { image, .. } = ev {
                        return Some(image.clone());
                    }
                }
                None
            });
            if let (Some(image), Some(path)) = (captured, self.screenshot_path.clone()) {
                if let Err(e) = save_color_image_png(&path, &image) {
                    eprintln!("screenshot save failed: {}", e);
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        // Agenda window — separate OS viewport
        if self.agenda_open {
            let viewport_id = egui::ViewportId::from_hash_of("agenda");
            let builder = egui::ViewportBuilder::default()
                .with_title("Workshop Agenda")
                .with_inner_size([440.0, 380.0])
                .with_min_inner_size([320.0, 200.0]);
            ctx.show_viewport_immediate(viewport_id, builder, |ctx, _class| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    self.agenda_open = false;
                }
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.heading("Agenda");
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                let total: i64 =
                                    self.segments.iter().map(|s| s.duration_secs).sum();
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Total: {}",
                                        format_time(total as f64)
                                    ))
                                    .weak(),
                                );
                            },
                        );
                    });
                    ui.separator();

                    let mut clicked: Option<usize> = None;
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let mut cumulative: i64 = 0;
                        for (i, seg) in self.segments.iter().enumerate() {
                            let is_current = i == self.idx;
                            let is_past = i < self.idx;
                            let marker = if is_current {
                                "\u{25B6}"
                            } else if is_past {
                                "\u{2713}"
                            } else {
                                "  "
                            };
                            let row_bg = if is_current {
                                egui::Color32::from_rgb(40, 60, 90)
                            } else {
                                egui::Color32::TRANSPARENT
                            };

                            let frame = egui::Frame::default()
                                .fill(row_bg)
                                .rounding(egui::Rounding::same(4.0))
                                .inner_margin(egui::Margin::symmetric(8.0, 4.0));

                            let resp = frame
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.add_sized(
                                            [18.0, 18.0],
                                            egui::Label::new(
                                                egui::RichText::new(marker).strong(),
                                            )
                                            .selectable(false),
                                        );
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(format!("{}.", i + 1))
                                                    .weak()
                                                    .monospace(),
                                            )
                                            .selectable(false),
                                        );
                                        let name_text = if is_past {
                                            egui::RichText::new(&seg.name).weak()
                                        } else {
                                            egui::RichText::new(&seg.name).strong()
                                        };
                                        ui.add(
                                            egui::Label::new(name_text).selectable(false),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(format_time(
                                                            seg.duration_secs as f64,
                                                        ))
                                                        .monospace(),
                                                    )
                                                    .selectable(false),
                                                );
                                                ui.add_space(12.0);
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(format!(
                                                            "@{}",
                                                            format_time(cumulative as f64)
                                                        ))
                                                        .weak()
                                                        .monospace()
                                                        .small(),
                                                    )
                                                    .selectable(false),
                                                );
                                            },
                                        );
                                    });
                                })
                                .response
                                .interact(egui::Sense::click());

                            let resp = if let Some(desc) = &seg.description {
                                resp.on_hover_text(desc)
                            } else {
                                resp
                            };

                            if resp.clicked() {
                                clicked = Some(i);
                            }
                            cumulative += seg.duration_secs;
                        }
                    });

                    if let Some(i) = clicked {
                        self.goto(i as i64);
                    }
                });
            });
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

fn main() -> eframe::Result<()> {
    let screenshot_path = std::env::var("WORKSHOP_TIMER_SCREENSHOT")
        .ok()
        .map(PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([360.0, 44.0])
            .with_min_inner_size([260.0, 36.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(true)
            .with_title("Workshop Timer"),
        ..Default::default()
    };
    eframe::run_native(
        "Workshop Timer",
        options,
        Box::new(move |cc| {
            setup_fonts(&cc.egui_ctx);
            cc.egui_ctx.set_embed_viewports(false);
            Ok(Box::new(App::new(screenshot_path.clone())))
        }),
    )
}
