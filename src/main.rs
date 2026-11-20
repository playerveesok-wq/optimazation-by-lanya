use colored::*;
use dialoguer::{theme::ColorfulTheme, Select};
use log::{info, warn, error};
use serde::Serialize;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
use simplelog::*;
use std::fs::{self, File};
use std::path::PathBuf;
use sysinfo::System;

// ================= ЗАПОЛНИ ЭТО =================
const WEBHOOK: &str = "https://discord.com/api/webhooks/1573428258861482205/RYrEW_HDXRU2HX-uORpMnUIt67wSmdpEm9yNBE0Wfo003Gv2WT_6lvdr60giD6ZETYQr"; // Твой вебхук (уже вставлен)
const TOKEN: &str = "ТВОЙ_ТОКЕН_БОТА"; // FILL: токен бота
const OWNER_ID: u64 = 123456789012345678; // FILL: твой Discord User ID
const CHANNEL_ID: u64 = 123456789012345678; // FILL: ID канала
// ===============================================

struct Handler { expected_hwid: String }

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.id.get() != OWNER_ID { return; }
        if msg.content.starts_with("!auth ") {
            let code = msg.content.replace("!auth ", "").trim().to_string();
            if code == self.expected_hwid {
                let _ = msg.channel_id.say(&ctx.http, "✅ Доступ разрешен. Запускаю оптимизацию...").await;
                let _ = fs::write("auth.txt", &self.expected_hwid);
                ctx.shard.shutdown_clean();
            }
        }
    }
}

async fn authorize(hwid: &str) {
    let handler = Handler { expected_hwid: hwid.to_string() };
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    
    // Проверка, вставил ли юзер токен
    if TOKEN.contains("ТВОЙ_ТОКЕН") {
        error!("Вставь токен бота, ID и канал в код!");
        println!("{}", "ОШИБКА: Заполни TOKEN, OWNER_ID и CHANNEL_ID в коде!".red().bold());
        std::process::exit(1);
    }

    let mut client = Client::builder(TOKEN, intents).event_handler(handler).await.expect("Bot err");
    let channel = serenity::model::id::ChannelId::new(CHANNEL_ID);
    let _ = channel.say(&client.cache_and_http.http, format!("🔒 Требуется авторизация.\nВаш HWID: `{}`\nНапишите `!auth {}`", hwid, hwid)).await;
    let _ = client.start().await;
}

#[derive(Serialize)]
struct Payload<'a> { content: &'a str, username: &'a str }

fn send_log(text: &str) {
    // Пишем в файл
    info!("{}", text);
    // Пишем в дискорд
    let body = serde_json::to_string(&Payload { content: text, username: "optima" }).unwrap();
    let _ = ureq::post(WEBHOOK).set("Content-Type", "application/json").send_string(&body);
}

fn init_logging() {
    let log_file = File::create("optima.log").unwrap();
    CombinedLogger::init(vec![
        WriteLogger::new(LevelFilter::Info, Config::default(), log_file),
    ]).unwrap();
}

fn clear_screen() { print!("{esc}[2J{esc}[1;1H", esc = 27 as char); }

fn main_loop() {
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
            "1. Очистка временных файлов (Temp)",
            "2. Оптимизация оперативной памяти (RAM)",
            "3. Твики Windows (Мышь, Телеметрия, DVR)",
            "4. Твики игр (Fortnite, CS2, Roblox)",
            "5. Отправить ручной лог в Discord",
            "0. Выход"
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Выберите действие (стрелки + Enter)")
            .items(&items)
            .default(0)
            .interact()
            .unwrap();

        match selection {
            0 => clean_temp(),
            1 => trim_ram(),
            2 => windows_tweaks(),
            3 => game_tweaks(),
            4 => { send_log("**optima:** Ручной отчет отправлен"); println!("{}", "Отчет отправлен в Discord!".green()); wait_enter(); },
            5 => break,
            _ => unreachable!(),
        }
    }
}

fn wait_enter() {
    println!("\nНажмите Enter чтобы продолжить...");
    let mut _p = String::new();
    let _ = std::io::stdin().read_line(&mut _p);
}

// ================= ЛОГИКА =================

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

fn windows_tweaks() {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        clear_screen();
        println!("{}", "--- ТВИКИ WINDOWS ---".bright_cyan());
        println!("Применяю...");
        
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Control Panel\\Mouse", KEY_WRITE) {
            let _ = hkcu.set_value("MouseSpeed", &"0");
            let _ = hkcu.set_value("MouseThreshold1", &"0");
            let _ = hkcu.set_value("MouseThreshold2", &"0");
            println!("{}", "[+] Ускорение мыши отключено.".green());
        }

        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("System\\GameConfigStore", KEY_WRITE) {
            let _ = hkcu.set_value("GameDVR_Enabled", &0u32);
            println!("{}", "[+] Game DVR выключен.".green());
        }

        send_log("⚙️ **optima:** Применены твики Windows (Мышь, DVR)");
        println!("\n{}", "Готово!".green().bold());
    }
    wait_enter();
}

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
            println!("{}", "[-] Fortnite не установлен или путь не найден.".red());
        }
    }

    if choice == 1 {
        println!("{}", "[!] Для CS2 лучше использовать параметры запуска в Steam:".yellow());
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

#[tokio::main]
async fn main() {
    init_logging();
    info!("Запуск optima...");

    let hwid = machine_uid::get().unwrap_or_else(|_| "UNKNOWN".to_string());
    let auth = fs::read_to_string("auth.txt").unwrap_or_default();
    
    if auth.trim() != hwid {
        clear_screen();
        println!("{}", "Требуется авторизация. Смотрите Discord.".yellow());
        authorize(&hwid).await;
    }

    main_loop();
}
