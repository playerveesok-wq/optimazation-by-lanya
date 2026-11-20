use colored::*;
use dialoguer::{theme::ColorfulTheme, Select};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use sysinfo::System;
use crossterm::event::{poll, read, Event};
use std::time::Duration;

// Твой вебхук
const WEBHOOK: &str = "https://discord.com/api/webhooks/1573428258861482205/RYrEW_HDXRU2HX-uORpMnUIt67wSmdpEm9yNBE0Wfo003Gv2WT_6lvdr60giD6ZETYQr";

#[derive(Serialize)]
struct Payload<'a> { content: &'a str, username: &'a str }

fn send_log(text: &str) {
    let body = serde_json::to_string(&Payload { content: text, username: "optima" }).unwrap();
    let _ = ureq::post(WEBHOOK).set("Content-Type", "application/json").send_string(&body);
}

fn clear_screen() { print!("{esc}[2J{esc}[1;1H", esc = 27 as char); }

fn wait_enter() {
    println!("\nНажмите Enter чтобы продолжить...");
    let mut _p = String::new();
    let _ = std::io::stdin().read_line(&mut _p);
}

// ================= ПУЛЬС ПК =================
fn pc_pulse() {
    let mut sys = System::new_all();
    let mut networks = sysinfo::Networks::new_with_refreshed_list();
    
    loop {
        sys.refresh_all();
        networks.refresh();
        
        clear_screen();
        println!("{}", "=========================================".bright_magenta());
        println!("{}", "             PC PULSE (LIVE)             ".bright_magenta().bold());
        println!("{}", "   Нажмите любую клавишу для выхода...   ".dimmed());
        println!("{}", "=========================================".bright_magenta());
        
        // Исправленная строчка под новую версию sysinfo
        let cpu_usage = sys.global_cpu_usage();
        let ram_used = sys.used_memory() / 1024 / 1024;
        let ram_total = sys.total_memory() / 1024 / 1024;
        let ram_percent = (ram_used as f64 / ram_total as f64) * 100.0;

        println!(" CPU Load: {:.1}%", cpu_usage);
        println!(" RAM:      {} MB / {} MB ({:.1}%)", ram_used, ram_total, ram_percent);
        
        // Сеть (суммарно за сессию)
        let mut rx = 0;
        let mut tx = 0;
        for (_name, data) in &networks {
            rx += data.total_received();
            tx += data.total_transmitted();
        }
        println!(" Net RX:   {:.2} MB", rx as f64 / 1024.0 / 1024.0);
        println!(" Net TX:   {:.2} MB", tx as f64 / 1024.0 / 1024.0);

        println!("{}", "-----------------------------------------".dimmed());
        println!("Топ процессов по ОЗУ:");
        
        let mut procs: Vec<_> = sys.processes().iter().collect();
        procs.sort_by(|a, b| b.1.memory().cmp(&a.1.memory()));
        for (pid, p) in procs.into_iter().take(5) {
            println!(" {:<25} {:>6} MB", p.name().to_string_lossy(), p.memory() / 1024 / 1024);
        }

        // Ждем 1 секунду или нажатия клавиши
        if poll(Duration::from_millis(1000)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = read() {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    break;
                }
            }
        }
    }
}

// ================= ТВИКИ =================

fn windows_tweaks_menu() {
    loop {
        clear_screen();
        println!("{}", "--- ТВИКИ WINDOWS ---".bright_cyan());
        let items = vec![
            "1. Базовые (Мышь, DVR, GameBar)",
            "2. Сетевые (Nagle, Throttling, TCP)",
            "3. Хардкор (Телеметрия, Приоритет CPU, Службы)",
            "0. Назад"
        ];

        let choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Выберите категорию")
            .items(&items)
            .default(0)
            .interact()
            .unwrap();

        match choice {
            0 => tweaks_basic(),
            1 => tweaks_network(),
            2 => tweaks_hardcore(),
            3 => break,
            _ => unreachable!(),
        }
    }
}

fn tweaks_basic() {
    clear_screen();
    println!("{}", "--- БАЗОВЫЕ ТВИКИ ---".bright_cyan());
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        
        // Мышь
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Control Panel\\Mouse", KEY_WRITE) {
            let _ = hkcu.set_value("MouseSpeed", &"0");
            let _ = hkcu.set_value("MouseThreshold1", &"0");
            let _ = hkcu.set_value("MouseThreshold2", &"0");
            println!("{}", "[+] Ускорение мыши отключено.".green());
        }
        
        // Game DVR
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("System\\GameConfigStore", KEY_WRITE) {
            let _ = hkcu.set_value("GameDVR_Enabled", &0u32);
            let _ = hkcu.set_value("GameDVR_FSEBehaviorMode", &2u32); // Force Fullscreen
            println!("{}", "[+] Game DVR и оптимизации выключены.".green());
        }
        send_log("⚙️ **optima:** Применены Базовые твики");
    }
    wait_enter();
}

fn tweaks_network() {
    clear_screen();
    println!("{}", "--- СЕТЕВЫЕ ТВИКИ ---".bright_cyan());
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        
        // Отключение Nagle (пинг в играх)
        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags("SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces", KEY_ALL_ACCESS) {
            for subkey in hklm.enum_keys().flatten() {
                if let Ok(iface) = hklm.open_subkey_with_flags(&subkey, KEY_WRITE) {
                    let _ = iface.set_value("TcpAckFrequency", &1u32);
                    let _ = iface.set_value("TCPNoDelay", &1u32);
                    let _ = iface.set_value("TcpDelAckTicks", &0u32);
                }
            }
            println!("{}", "[+] Nagle's Algorithm отключен (снижение пинга).".green());
        }

        // Network Throttling
        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile", KEY_WRITE) {
            let _ = hklm.set_value("NetworkThrottlingIndex", &0xffffffffu32);
            let _ = hklm.set_value("SystemResponsiveness", &10u32);
            println!("{}", "[+] Network Throttling отключен.".green());
        }
        send_log("🌐 **optima:** Применены Сетевые твики");
    }
    wait_enter();
}

fn tweaks_hardcore() {
    clear_screen();
    println!("{}", "--- ХАРДКОРНЫЕ ТВИКИ ---".bright_cyan());
    println!("{}", "[!] Требуются права Администратора!".yellow());
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        
        // Отключение Телеметрии
        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE).create_subkey_with_flags("SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection", KEY_WRITE) {
            let _ = hklm.0.set_value("AllowTelemetry", &0u32);
            println!("{}", "[+] Телеметрия отключена.".green());
        } else {
            println!("{}", "[-] Нет прав для отключения телеметрии (Запусти от Админа).".red());
        }

        // Приоритет CPU для игр (Win32PrioritySeparation)
        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags("SYSTEM\\CurrentControlSet\\Control\\PriorityControl", KEY_WRITE) {
            let _ = hklm.set_value("Win32PrioritySeparation", &38u32); // 38 = 26 hex (Short, Fixed, High)
            println!("{}", "[+] Приоритет CPU настроен под игры.".green());
        }

        // Отключение Xbox Game Bar
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).create_subkey_with_flags("Software\\Microsoft\\GameBar", KEY_WRITE) {
            let _ = hkcu.0.set_value("UseNexusForGameBarEnabled", &0u32);
            println!("{}", "[+] Xbox Game Bar отключен.".green());
        }

        send_log("🔥 **optima:** Применены Хардкорные твики");
    }
    wait_enter();
}

// ================= ИГРЫ =================

fn game_tweaks() {
    clear_screen();
    println!("{}", "--- ТВИКИ ИГР ---".bright_cyan());
    
    let games = vec!["Fortnite", "CS2", "Roblox", "Назад"];
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Выберите игру")
        .items(&games)
        .default(0)
        .interact()
        .unwrap();

    if choice == 3 { return; }

    if choice == 0 {
        let mut path = dirs::data_local_dir().unwrap();
        path.push("FortniteGame\\Saved\\Config\\WindowsClient\\GameUserSettings.ini");
        if path.exists() {
            let mut content = fs::read_to_string(&path).unwrap_or_default();
            if content.contains("FrameRateLimit") {
                content = content.replace("FrameRateLimit=60.000000", "FrameRateLimit=165.000000");
                content = content.replace("bUseVSync=True", "bUseVSync=False");
                let _ = fs::write(&path, content);
                println!("{}", "[+] Fortnite: V-Sync выкл, FPS лимит поднят.".green());
                send_log("🎮 **optima:** Применены твики Fortnite");
            } else {
                println!("{}", "[-] Не нашел нужных строк в конфиге Fortnite.".red());
            }
        } else {
            println!("{}", "[-] Fortnite не установлен.".red());
        }
    }

    if choice == 1 {
        println!("{}", "[!] Для CS2 параметры запуска в Steam:".yellow());
        println!("-novid -high -threads [кол-во ядер] +fps_max 0");
        send_log("🎮 **optima:** Выданы рекомендации для CS2");
    }

    if choice == 2 {
        let mut path = dirs::data_local_dir().unwrap();
        path.push("Roblox\\GlobalBasicSettings_13.xml");
        if path.exists() {
            let mut content = fs::read_to_string(&path).unwrap_or_default();
            if content.contains("<int name=\"FramerateCap\">") {
                let re = regex::Regex::new(r#"<int name="FramerateCap">\d+</int>"#).unwrap();
                content = re.replace(&content, "<int name=\"FramerateCap\">240</int>").to_string();
                let _ = fs::write(&path, content);
                println!("{}", "[+] Roblox: FPS кэп поднят до 240.".green());
                send_log("🎮 **optima:** Применены твики Roblox");
            }
        } else {
            println!("{}", "[-] Roblox не найден.".red());
        }
    }
    wait_enter();
}

// ================= ОЧИСТКА =================

fn clean_temp() {
    clear_screen();
    println!("{}", "--- ОЧИСТКА TEMP ---".bright_cyan());
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
    let msg = format!("Очищено: {} MB", freed / 1024 / 1024);
    println!("{}", msg.green());
    send_log(&format!("🧹 **optima:** {}", msg));
    wait_enter();
}

fn trim_ram() {
    clear_screen();
    println!("{}", "--- ОПТИМИЗАЦИЯ RAM ---".bright_cyan());
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
        let msg = format!("Тримминг ОЗУ: {} процессов", trimmed);
        println!("{}", msg.green());
        send_log(&format!("🧠 **optima:** {}", msg));
    }
    wait_enter();
}

// ================= MAIN =================

fn main() {
    let mut sys = System::new_all();
    sys.refresh_all();
    let total_ram = sys.total_memory() / 1024 / 1024;
    let cpu = sys.cpus()[0].brand();

    loop {
        clear_screen();
        println!("{}", "=========================================".bright_cyan());
        println!("{}", "       OPTIMA - SYSTEM OPTIMIZER         ".bright_cyan().bold());
        println!("{}", "=========================================".bright_cyan());
        println!(" CPU: {} | RAM: {} MB", cpu, total_ram);
        println!("{}", "-----------------------------------------".dimmed());

        if total_ram < 8192 {
            println!("{}", "[!] У вас мало ОЗУ (<8GB). Рекомендуется закрыть браузер перед играми.".yellow());
        } else {
            println!("{}", "[+] Железо в норме. Можно применять тяжелые твики.".green());
        }
        println!("{}", "-----------------------------------------".dimmed());

        let items = vec![
            "1. Пульс ПК (Live Мониторинг)",
            "2. Очистка временных файлов (Temp)",
            "3. Оптимизация оперативной памяти (RAM)",
            "4. Твики Windows (Базовые, Сеть, Хардкор)",
            "5. Твики игр (Fortnite, CS2, Roblox)",
            "6. Отправить ручной лог в Discord",
            "0. Выход"
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Выберите действие (стрелки + Enter)")
            .items(&items)
            .default(0)
            .interact()
            .unwrap();

        match selection {
            0 => pc_pulse(),
            1 => clean_temp(),
            2 => trim_ram(),
            3 => windows_tweaks_menu(),
            4 => game_tweaks(),
            5 => { send_log("**optima:** Ручной отчет отправлен"); println!("{}", "Отчет отправлен в Discord!".green()); wait_enter(); },
            6 => break,
            _ => unreachable!(),
        }
    }
}
