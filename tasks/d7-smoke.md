# D7 Smoke Test Checklist — 1.0 MAS Build

Manual click-through to be run before App Store submission. Maps to
spec §6-1 (T1–T10) plus migration edge cases from D5
(`docs/superpowers/specs/2026-04-26-mas-readiness-design.md` §4-3).

The spec's T1/T2/T10 wording uses the rev-1 "데이터 가져오기 (copy-import)"
framing. We shipped rev-2 (Option 1, security-scoped bookmark), so
expected button labels are "폴더 선택" / "나중에" / "다른 폴더 연결".

---

## 0. Setup

- [ ] `bash scripts/check-signing.sh` → 7 ✓
- [ ] `bash scripts/build-mas.sh` succeeds → `dist-mas/Hearth-1.0.0.pkg`
- [ ] `xcrun altool --validate-app -f dist-mas/Hearth-1.0.0.pkg ...` returns no errors
- [ ] `spctl --assess --type install dist-mas/Hearth-1.0.0.pkg` accepts
- [ ] Install the .app to `/Applications/Hearth.app`. **Note:** plain
      `sudo installer -pkg ... -target /` does NOT work for the MAS-signed
      pkg locally — the system writes a receipt but skips the .app payload
      (Console: `MobileInstallation` + `Trust evaluate failure: [leaf
      ExtendedKeyUsage]`). Apple Distribution + 3rd Party Installer cert
      combo is only honored by App Store / TestFlight. Local workaround:
      ```bash
      WORK=$(mktemp -d)
      pkgutil --expand-full dist-mas/Hearth-1.0.0-N.pkg "$WORK/expanded"
      APP=$(find "$WORK/expanded" -name "Hearth.app" -maxdepth 5 -type d | head -1)
      sudo rm -rf /Applications/Hearth.app
      sudo ditto "$APP" /Applications/Hearth.app
      ```
      The inner pkg name (e.g. `com.codewithgenie.hearth.pkg`) varies, so
      glob with `find` rather than hard-coding the path.
      Sandbox / entitlements / signing all behave the same as the
      installed pkg would. For the *real* end-to-end install path use
      TestFlight after upload.
- [ ] **Reset state before each test run that requires "first launch":**
  ```bash
  defaults delete com.codewithgenie.hearth hearth.dataDirBookmark 2>/dev/null
  defaults delete com.codewithgenie.hearth hearth.migrationDismissed 2>/dev/null
  rm -rf ~/Library/Containers/com.codewithgenie.hearth
  ```

---

## 1. T1–T10 sandbox matrix (spec §6-1)

### T1 — First launch, no 0.x data
- [ ] Reset state (see Setup)
- [ ] Pre-flight: `~/Library/Application Support/com.codewithgenie.hearth/` does not exist
- [ ] Launch Hearth.app
- [ ] **Migration wizard appears** within ~2s of webview mount
- [ ] Click "나중에"
- [ ] Wizard closes; main UI loads in degraded mode (container fallback)
- [ ] Settings → 일반 → "데이터 폴더" shows "현재 기본 위치(샌드박스 컨테이너)에서 동작 중입니다"

### T2 — First launch, 0.x data present
- [x] Reset state (see Setup), but leave `~/Library/Application Support/com.codewithgenie.hearth/data.db` from a 0.x install in place
      (covered via working copy at `~/Desktop/hearth-m4-test/` to protect real data)
- [x] Launch Hearth.app
- [x] Wizard appears
- [x] Click "폴더 선택"
- [x] **NSOpenPanel opens pre-pointed at `~/Library/Application Support/com.codewithgenie.hearth/`** (not the container)
- [x] **NSOpenPanel only allows folder selection** (canChooseFiles=false; if you can pick `data.db` directly, that's a bug)
- [x] Confirm folder; wizard transitions to "지금 재시작" prompt with the resolved path shown
- [x] Click "지금 재시작"
- [x] App restarts; on second boot **the wizard does NOT appear** and existing 0.x projects/memos/schedules are visible

### T3 — Second launch
- [ ] (Continuation of T1 or T2 path)
- [ ] Quit and re-launch Hearth.app
- [ ] No wizard, app boots straight to main UI
- [ ] Console.app filter `process: Hearth` shows no `bookmark resolve failed` / `startAccessingSecurityScopedResource returned false` / `pre-1.0 snapshot` errors

### T4 — Quick Capture ⌃⇧H
- [x] Switch focus to a non-Hearth app (e.g. Safari)
- [x] Press ⌃⇧H
- [x] Quick Capture overlay appears on top
- [x] Type a memo, hit Enter
- [x] Memo saved; Hearth memo board shows it after switching back

### T5 — Backup folder picker
- [ ] Settings → 백업 → "백업 폴더 변경"
- [ ] Pick `~/Desktop` or another arbitrary folder
- [ ] Trigger a backup ("지금 백업")
- [ ] Backup file appears in chosen folder

### T6 — OpenAI key + ⌘K
- [ ] Settings → AI → enter a valid OpenAI API key
- [ ] Press ⌘K to open command palette
- [ ] Type a natural-language query ("내일 회의 추가해줘" 등)
- [ ] Returns a sensible tool-call response

### T7 — Schedule notification
- [x] Create a schedule for ~2 minutes in the future with "5분 전 알림" toggle ON
- [x] First run: macOS notification permission prompt appears → grant
- [x] Wait until the trigger time
- [x] System notification fires (with sound — `.sound("default")` added in this session)

### T8 — Finder reveal
- [ ] In a project with a path set, right-click → "Finder에서 열기"
- [ ] Finder opens with the file/folder highlighted (not just the parent dir)

### T9 — App Store deep link
- [ ] Settings → About (or wherever the App Store link lives) → click "App Store에서 보기"
- [ ] App Store.app opens (even if the listing is empty pre-launch, the `macappstore://` scheme handler should fire — no error dialog)

### T10 — Re-entry from Settings
- [ ] Settings → 일반 → "데이터 폴더" → "다른 폴더 연결" (or "데이터 폴더 연결" if no bookmark yet)
- [ ] NSOpenPanel opens
- [ ] Pick a different folder
- [ ] Inline "지금 재시작" button appears below
- [ ] Click it; app restarts; new folder is now the canonical DB location

---

## 2. Migration edge cases (D5-specific)

### M1 — Dismiss → Settings re-entry round-trip
- [ ] Reset state
- [ ] Launch → wizard → "나중에"
- [ ] Quit and relaunch
- [ ] **Wizard does NOT reappear** (dismissed marker honored)
- [ ] Settings → 일반 → "데이터 폴더 연결" → pick folder → restart
- [ ] After restart, status panel shows the resolved path

### M2 — Stale bookmark (folder renamed)
- [x] Pick a folder via the wizard, complete restart
- [x] Quit Hearth
- [x] Rename the chosen folder in Finder
- [x] Launch Hearth
- [x] App boots without re-prompting (NSURL bookmark resolves the renamed folder by inode)
- [ ] ~~Settings → 일반 → 데이터 폴더 shows "폴더가 이동된 것을 감지했어요. 자동으로 재연결되었습니다."~~
      (UX message never implemented — Settings just shows the new resolved path. Defer to 1.1; not a ship-blocker since the auto-reconnect itself works.)
- [x] Console.app: no `stale bookmark refresh failed` errors (refresh path was fixed in #31)

### M3 — Bookmark folder deleted (resolution failure)
- [x] Pick a folder via the wizard, complete restart
- [x] Quit Hearth
- [x] Delete the chosen folder in Finder
- [x] Launch Hearth
- [x] Console.app shows `bookmark resolve failed: ...`
- [x] **Wizard reappears** (poison-pill blob was cleared per #31 fix)
- [x] Pick a new folder, restart, app works

### M4 — Pre-1.0 snapshot
- [x] Pick a folder containing a real 0.9.x `data.db` (e.g. your existing OSS install)
      (used `~/Desktop/hearth-m4-test/` working copy of `com.newturn2017.hearth/data.db`, projects=36 memos=10 schedules=46)
- [x] After the post-pick restart and first 1.0 boot, that folder should now contain `data.db.pre-1.0.bak`
- [x] `sqlite3 data.db "PRAGMA user_version;"` returns `1`
- [x] `sqlite3 data.db.pre-1.0.bak "PRAGMA user_version;"` returns `0`
- [ ] Re-launch Hearth: `data.db.pre-1.0.bak` mtime does NOT change (no re-snapshot)

### M5 — Hot bookmark + CLI co-write
- [ ] Hearth running with bookmark active
- [ ] In a separate terminal, install or invoke `hearth-cli` (homebrew/cargo) pointing at the same canonical DB
- [ ] Create a memo via CLI
- [ ] Hearth UI auto-refreshes (watcher fires on file change) — memo appears
- [ ] Reverse direction: create a memo in Hearth UI; CLI `hearth-cli memo list` sees it

---

## 3. DoD verification (spec §6-5)

Things that can be verified by command, not by clicking:

- [ ] `rg -i 'updater|autostart' src src-tauri` → only safe references (anti-string in #26's settings card copy, etc.)
- [ ] `cargo build --release -p hearth-app` → 0 warnings
- [ ] `/usr/libexec/PlistBuddy -c "Print :LSMinimumSystemVersion" "/Applications/Hearth.app/Contents/Info.plist"` → `11.0`
- [ ] First-launch debug log shows DB path under `~/Library/Application Support/com.codewithgenie.hearth/`, NOT `~/Library/Containers/...` (after completing the wizard)
- [ ] After macOS reboot: launch Hearth — no NSOpenPanel reprompt, bookmark resolves silently
- [ ] Calendar reminder for 2027-04-XX (provisioning profile renewal)

---

## 4. Bugs to watch for (regressions of #31 fixes)

If any of these symptoms appear, suspect a regression:

| symptom | likely regression | check |
|---|---|---|
| NSOpenPanel opens at `~/Library/Containers/...` | Bug 3 | `cmd_migration.rs::choose_folder` — initial_dir from `$HOME` |
| Picking `data.db` produces a SQLite error | Bug 2 | `setCanChooseFiles` flipped back to `true` |
| Stale-bookmark recovery emits `bookmarkDataWithOptions failed` repeatedly | Bug 1 | `refresh_stale_bookmark` taking a path string instead of `&NSURL` |
| `startAccessingSecurityScopedResource returned false` on every boot | Bug 4 | `clear_bookmark_blob` not called on startAccess fail |
| Wizard text mentions `com.newturn2017.hearth` | Bug 5 | hardcoded bundle id |
| Wizard never appears even with no bookmark + no dismiss marker | Belt-and-suspenders gone | `MigrationWizard.tsx` mount-time `getDataFolderStatus` query missing |
| `database is locked` errors when CLI writes simultaneously | busy_timeout removed | `core/db.rs` busy_timeout=5s |
