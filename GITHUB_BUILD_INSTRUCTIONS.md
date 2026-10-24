# AutoSubs - GitHub Actions Build Instructions

Tento repozitář obsahuje nastavené **GitHub Actions workflow** pro automatické buildování AutoSubs pro:
- **Windows** (`.exe`, `.msi`, portable)
- **Linux** (`.AppImage`, `.deb`, portable)
- **macOS** (`.dmg`, `.app` - Intel i Apple Silicon)

---

## 🚀 Jak to zprovoznit (5 minut)

### 1. Vytvořte prázdný repozitář na GitHubu
- Jděte na https://github.com/new
- Název: `auto-subs` (nebo jaký chcete)
- **Nezaškrtávejte** "Add a README file", ".gitignore", "license"
- Klikněte **Create repository**

### 2. Nahrajte kód (z terminálu ve složce projektu)

```bash
cd "C:\Users\Tobias\auto-subs"

# Inicializace git (pokud ještě není)
git init

# Přidání všech souborů
git add .

# První commit
git commit -m "Add reference video style transfer feature"

# Hlavní větev
git branch -M main

# Připojení k vašemu repozitáři (nahraďte VASE_UCET)
git remote add origin https://github.com/VASE_UCET/auto-subs.git

# Nahrání
git push -u origin main
```

### 3. Build se spustí automaticky
- Jděte na **GitHub → Actions** záložku
- Uvidíte běžící workflow: **Build Windows**, **Build Linux**, **Build macOS**
- První build trvá **~10-15 minut** (stahuje závislosti)

### 4. Stažení výsledných souborů

#### Z Actions (pro každý commit):
1. Klikněte na dokončený workflow run
2. Do sekce **Artifacts** stáhněte:
   - `AutoSubs-Windows-Installer.zip` → `.exe` + `.msi` instalátory
   - `AutoSubs-Windows-Portable.zip` → portable `autosubs.exe`
   - `AutoSubs-Linux-AppImage.zip` / `AutoSubs-Linux-Deb.zip`
   - `AutoSubs-macOS-Intel-DMG.zip` / `AutoSubs-macOS-ARM64-DMG.zip`

#### Z Releases (pouze při tagování verze):
```bash
# Vytvoření release tagu
git tag v3.12.0
git push origin v3.12.0
```
Toto vytvoří **GitHub Release** s připravenými instalátory ke stažení pro uživatele.

---

## 📁 Struktura workflow souborů

```
.github/
└── workflows/
    ├── build-windows.yml    # Windows build (NSIS + MSI + portable)
    ├── build-linux.yml      # Linux build (AppImage + deb + portable)
    └── build-macos.yml      # macOS build (DMG + .app pro Intel i ARM)
```

---

## ⚙️ Konfigurace (volitelné)

### Podepisování Windows instalátoru (Code Signing)
Pro produkční release byste měli nastavit v **GitHub → Settings → Secrets and variables → Actions**:

| Secret | Popis |
|--------|-------|
| `TAURI_SIGNING_PRIVATE_KEY` | Base64 encoded .pfx certifikát |
| `TAURI_SIGNING_KEY_PASSWORD` | Heslo k certifikátu |

Bez toho bude instalátor **nepodepsaný** (Windows zobrazí "Unknown publisher" varování).

---

## 🐛 Řešení problémů

### Build selže na "cargo metadata"
- Zkontrolujte, že `AutoSubs-App/src-tauri/Cargo.toml` existuje
- Ujistěte se, že `Cargo.lock` je v commitu (`git add AutoSubs-App/src-tauri/Cargo.lock`)

### Build selže na frontend dependencies
- Zkontrolujte `AutoSubs-App/package-lock.json` je v commitu
- Spusťte lokálně `npm ci` v `AutoSubs-App` pro ověření

### macOS build selže na codesign
- Pro vývoj stačí `npm run build:mac:arm64` / `x86_64` lokálně
- Na GitHubu se podepisování přeskočí pokud nejsou nastaveny certifikáty

---

## 📦 Co dostanete

| Platforma | Soubory |
|-----------|---------|
| **Windows** | `AutoSubs_3.12.0_x64-setup.exe`, `AutoSubs_3.12.0_x64.msi`, `autosubs.exe` (portable) |
| **Linux** | `AutoSubs_3.12.0_amd64.AppImage`, `autosubs_3.12.0_amd64.deb`, `autosubs` (portable) |
| **macOS Intel** | `AutoSubs_3.12.0_x64.dmg`, `AutoSubs.app` |
| **macOS ARM** | `AutoSubs_3.12.0_aarch64.dmg`, `AutoSubs.app` |

---

## 🔄 Aktualizace kódu

Pro další změny stačí:
```bash
git add .
git commit -m "Popis změny"
git push
```
GitHub Actions se spustí sám a vybuilduje novou verzi.

---

## 💡 Tip: Rychlý test bez GitHubu

Pokud chcete testovat build lokálně **bez instalace Rustu**, použijte **GitHub Codespaces**:
1. Na GitHubu klikněte **Code → Codespaces → Create codespace on main**
2. V terminálu Codespaces: `cd AutoSubs-App && npm run build:linux`
3. Hotový `.AppImage` bude v `src-tauri/target/release/bundle/appimage/`