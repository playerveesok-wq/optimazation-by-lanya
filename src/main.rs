#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Скрывает консоль

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use sysinfo::System;

const WEBHOOK: &str = "https://discord.com/api/webhooks/1573428258861482205/RYrEW_HDXRU2HX-uORpMnUIt67wSmdpEm9yNBE0Wfo003Gv2WT_6lvdr60giD6ZETYQr";

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    send_logs: bool,
    accent_color: [u8; 3],
}

impl Default for Config {
    fn default() -> Self {
        Self { send_logs: true, accent_color: [0, 150, 255] }
    }
}

impl Config {
    fn load() -> Self {
        fs::read_to_string("config.json").ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    fn save(&self) {
        let _ = fs::write("config.json", serde_json::to_string_pretty(self).unwrap());
    }
}

#[derive(Serialize)]
struct Payload<'a> { content: &'a str, username: &'a str }

fn send_log(conf: &Config, text: &str) {
    if !conf.send_logs { return; }
    let body = serde_json::to_string(&Payload { content: text, username: "optima" }).unwrap();
    let _ = ureq::post(WEBHOOK).set("Content-Type", "application/json").send_string(&body);
}

#[derive(PartialEq)]
enum Tab { Dashboard, PingBooster, Tweaks, Settings }

struct OptimaApp {
    tab: Tab,
    sys: System,
    config: Config,
    log_history: Vec<String>,
    ping_ms: u128,
}

impl OptimaApp {
    fn new() -> Self {
        Self {
            tab: Tab::Dashboard,
            sys: System::new_all(),
            config: Config::load(),
            log_history: vec!["Программа запущена.".into()],
            ping_ms: 0,
        }
    }

    fn log(&mut self, msg: String) {
        self.log_history.push(msg.clone());
        send_log(&self.config, &msg);
    }
}

impl eframe::App for OptimaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sys.refresh_all();

        let mut visuals = egui::Visuals::dark();
        let accent = egui::Color32::from_rgb(self.config.accent_color[0], self.config.accent_color[1], self.config.accent_color[2]);
        visuals.selection.bg_fill = accent;
        visuals.widgets.hovered.bg_fill = accent.linear_multiply(0.4);
        visuals.widgets.active.bg_fill = accent;
        ctx.set_visuals(visuals);

        egui::SidePanel::left("menu").resizable(false).default_width(180.0).show(ctx, |ui| {
            ui.add_space(20.0);
            ui.heading(egui::RichText::new("OPTIMA").color(accent).strong().size(28.0));
            ui.label(egui::RichText::new("system optimizer").size(10.0).color(egui::Color32::GRAY));
            ui.separator();
            ui.add_space(10.0);

            if ui.selectable_label(self.tab == Tab::Dashboard, "📊 Дашборд").clicked() { self.tab = Tab::Dashboard; }
            if ui.selectable_label(self.tab == Tab::PingBooster, "⚡ Бустер Пинга").clicked() { self.tab = Tab::PingBooster; }
            if ui.selectable_label(self.tab == Tab::Tweaks, "⚙️ Твики Системы").clicked() { self.tab = Tab::Tweaks; }
            if ui.selectable_label(self.tab == Tab::Settings, "🛠 Настройки").clicked() { self.tab = Tab::Settings; }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.tab {
                Tab::Dashboard => self.draw_dashboard(ui, accent),
                Tab::PingBooster => self.draw_ping_booster(ui, accent),
                Tab::Tweaks => self.draw_tweaks(ui),
                Tab::Settings => self.draw_settings(ui),
            }
        });
    }
}

impl OptimaApp {
    fn draw_dashboard(&mut self, ui: &mut egui::Ui, accent: egui::Color32) {
        ui.heading("Дашборд");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Загрузка ЦП:");
                ui.heading(format!("{:.1}%", self.sys.global_cpu_usage()));
            });
            ui.add_space(40.0);
            ui.vertical(|ui| {
                ui.label("Оперативная память:");
                let used = self.sys.used_memory() / 1024 / 1024;
                let total = self.sys.total_memory() / 1024 / 1024;
                ui.heading(format!("{} / {} MB", used, total));
            });
        });

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);
        
        ui.label("Быстрые действия:");
        ui.horizontal(|ui| {
            if ui.button("🧹 Очистить Temp").clicked() {
                let freed = clean_temp();
                self.log(format!("🧹 Очищено: {} MB", freed));
            }
            if ui.button("🧠 Тримминг ОЗУ").clicked() {
                let count = trim_ram();
                self.log(format!("🧠 Тримминг ОЗУ: {} процессов", count));
            }
        });

        ui.add_space(20.0);
        ui.label("История логов:");
        egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
            for log in self.log_history.iter().rev() {
                ui.label(egui::RichText::new(log).color(egui::Color32::LIGHT_GRAY));
            }
        });
    }

    fn draw_ping_booster(&mut self, ui: &mut egui::Ui, accent: egui::Color32) {
        ui.heading("Бустер Пинга");
        ui.label("Оптимизация сетевых протоколов Windows для снижения задержек.");
        ui.add_space(20.0);

        ui.group(|ui| {
            ui.heading(egui::RichText::new(format!("Текущий пинг: {} ms", self.ping_ms)).color(accent));
            if ui.button("Проверить Пинг").clicked() {
                let start = std::time::Instant::now();
                let _ = ureq::get("http://1.1.1.1").call(); 
                self.ping_ms = start.elapsed().as_millis();
            }
        });

        ui.add_space(20.0);
        if ui.button("⚡ Применить сетевые твики (Nagle, Throttling)").clicked() {
            apply_network_tweaks();
            self.log("🌐 Применены сетевые твики для пинга".into());
        }
    }

    fn draw_tweaks(&mut self, ui: &mut egui::Ui) {
        ui.heading("Твики Системы");
        ui.label(egui::RichText::new("Внимание: для применения твиков запустите от имени Администратора!").color(egui::Color32::YELLOW));
        ui.add_space(10.0);
        
        if ui.button("🖱 Отключить ускорение мыши").clicked() { tweak_mouse(); self.log("🖱 Ускорение мыши отключено".into()); }
        if ui.button("🎮 Отключить Xbox Game Bar").clicked() { tweak_dvr(); self.log("🎮 Game Bar отключен".into()); }
        if ui.button("🔥 Отключить Телеметрию").clicked() { tweak_telemetry(); self.log("🔥 Телеметрия отключена".into()); }
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Настройки");
        ui.add_space(10.0);
        
        let mut changed = false;
        if ui.checkbox(&mut self.config.send_logs, "Отправлять логи в Discord (Webhook)").changed() { changed = true; }
        
        ui.add_space(10.0);
        ui.label("Цвет акцента интерфейса:");
        if ui.color_edit_button_srgb(&mut self.config.accent_color).changed() { changed = true; }

        if changed { self.config.save(); }
    }
}

// ================= ФУНКЦИИ =================

fn clean_temp() -> u64 {
    let mut freed = 0;
    let mut targets = vec![];
    if let Ok(p) = std::env::var("TEMP") { targets.push(PathBuf::from(p)); }
    if let Ok(p) = std::env::var("TMP") { targets.push(PathBuf::from(p)); }
    if let Ok(win) = std::env::var("WINDIR") { targets.push(PathBuf::from(&win).join("Temp")); }

    for path in targets {
        if !path.exists() { continue; }
        if let Ok(rd) = fs::read_dir(path) {
            for entry in rd.flatten() {
                let p = entry.path();
                if let Ok(md) = entry.metadata() {
                    if md.is_file() { freed += md.len(); let _ = fs::remove_file(&p); } 
                    else if md.is_dir() { let _ = fs::remove_dir_all(&p); }
                }
            }
        }
    }
    freed / 1024 / 1024
}

fn trim_ram() -> u64 {
    #[cfg(windows)]
    {
        use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA};
        use windows::Win32::Foundation::CloseHandle;
        let s = System::new_all();
        let mut trimmed = 0;
        for (pid, _) in s.processes() {
            unsafe {
                if let Ok(h) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA, false, pid.as_u32()) {
                    if EmptyWorkingSet(h).is_ok() { trimmed += 1; }
                    let _ = CloseHandle(h);
                }
            }
        }
        trimmed
    }
    #[cfg(not(windows))]
    { 0 }
}

fn apply_network_tweaks() {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags("SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces", KEY_ALL_ACCESS) {
            for subkey in hklm.enum_keys().flatten() {
                if let Ok(iface) = hklm.open_subkey_with_flags(&subkey, KEY_WRITE) {
                    let _ = iface.set_value("TcpAckFrequency", &1u32);
                    let _ = iface.set_value("TCPNoDelay", &1u32);
                }
            }
        }
    }
}

fn tweak_mouse() {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Control Panel\\Mouse", KEY_WRITE) {
            let _ = hkcu.set_value("MouseSpeed", &"0");
            let _ = hkcu.set_value("MouseThreshold1", &"0");
            let _ = hkcu.set_value("MouseThreshold2", &"0");
        }
    }
}

fn tweak_dvr() {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("System\\GameConfigStore", KEY_WRITE) {
            let _ = hkcu.set_value("GameDVR_Enabled", &0u32);
        }
    }
}

fn tweak_telemetry() {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE).create_subkey_with_flags("SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection", KEY_WRITE) {
            let _ = hklm.0.set_value("AllowTelemetry", &0u32);
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([850.0, 600.0])
            .with_min_inner_size([600.0, 400.0])
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../icon.ico")).unwrap_or_default()),
        ..Default::default()
    };
    
    eframe::run_native(
        "optima",
        options,
        Box::new(|_cc| Box::new(OptimaApp::new())),
    )
}
