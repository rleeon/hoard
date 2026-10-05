; hoardd (ADR 0021) sigue vivo tras cerrar la app — install/uninstall pisan su
; propio .exe en AppData\Local\Hoard mientras el proceso lo tiene abierto, y
; NSIS falla con "Error opening file for writing". Tauri solo cierra el exe
; principal por su cuenta; los sidecars hay que matarlos a mano aquí.
;
; The kill order is the fix, not decoration. A client that loses the socket
; starts a service ("spawn if absent", Slice 4), so killing `hoardd.exe` while
; `hoard-desktop.exe` is still up gets it started again from the old binary
; within ~2s — and NSIS then hits a locked file. Clients die first, the service
; last. `hoard_agent::install::Swap` covers the clients we can't name here.
;
; Whatever we stop, we start again: this installer runs silently from the
; updater (`/S`), so nothing else would. Without the post-install half, a
; machine that updates in the background is left with no sync service until
; someone opens the app.
!macro NSIS_HOOK_PREINSTALL
  Push $0
  Push $1
  ; `hoard_agent::install::Swap`, written from here because a hand-run installer
  ; has no updater to write it: whoever opens the app while this runs would
  ; otherwise start a service off the binaries being overwritten, and that is
  ; the file lock. Both paths because the state dir moved from Local to Roaming
  ; and an old install can still be answering with the old one.
  CreateDirectory "$APPDATA\hoard\hoard\data"
  FileOpen $1 "$APPDATA\hoard\hoard\data\swapping-binaries" w
  FileClose $1
  CreateDirectory "$LOCALAPPDATA\hoard\hoard\data"
  FileOpen $1 "$LOCALAPPDATA\hoard\hoard\data\swapping-binaries" w
  FileClose $1
  ExecWait 'taskkill /F /IM hoard-desktop.exe' $0
  StrCmp $0 "0" 0 hoard_pre_no_app
    FileOpen $1 "$TEMP\hoard-restart-app.flag" w
    FileClose $1
  hoard_pre_no_app:
  ExecWait 'taskkill /F /IM hoard-screen.exe'
  ExecWait 'taskkill /F /IM hoardd.exe' $0
  StrCmp $0 "0" 0 hoard_pre_no_service
    FileOpen $1 "$TEMP\hoard-restart-service.flag" w
    FileClose $1
  hoard_pre_no_service:
  ; taskkill returns before the process is actually gone; the handles go with
  ; it a moment later.
  Sleep 1500
  Pop $1
  Pop $0
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Delete "$APPDATA\hoard\hoard\data\swapping-binaries"
  Delete "$LOCALAPPDATA\hoard\hoard\data\swapping-binaries"
  ; The overlay no longer ships, and an install over an older one leaves its
  ; exe behind in the install dir.
  Delete "$INSTDIR\hoard-screen.exe"
  ; Service first, same as everywhere else: the app expects it to be there.
  ; The uninstall marker asks for it too: an installer run by hand over an
  ; older copy uninstalls that copy first, login start goes with it, and the
  ; service is what puts it back (`autostart::reclaim_after_reinstall`).
  IfFileExists "$APPDATA\hoard\hoard\config\login-start-removed" hoard_post_start_service
  IfFileExists "$TEMP\hoard-restart-service.flag" 0 hoard_post_no_service
  hoard_post_start_service:
    Delete "$TEMP\hoard-restart-service.flag"
    Exec '"$INSTDIR\hoardd.exe"'
  hoard_post_no_service:
  IfFileExists "$TEMP\hoard-restart-app.flag" 0 hoard_post_no_app
    Delete "$TEMP\hoard-restart-app.flag"
    Exec '"$INSTDIR\hoard-desktop.exe"'
  hoard_post_no_app:
!macroend

; Uninstall kills in the same order and brings nothing back.
!macro NSIS_HOOK_PREUNINSTALL
  ExecWait 'taskkill /F /IM hoard-desktop.exe'
  ExecWait 'taskkill /F /IM hoardd.exe'
  Sleep 1500
!macroend

; Tauri's uninstaller only removes what it put in $INSTDIR. HoardSetup puts the
; core in $LOCALAPPDATA\hoard\bin, a subfolder of $INSTDIR on a
; case-insensitive disk that its non-recursive RMDir leaves standing, so after
; "uninstall" from Settings > Apps the HoardSync task still started that
; `hoardd` at every logon, the
; install manifest still listed the app, and the updater reinstalled it,
; desktop shortcut included, with the next release.
;
; Not in update mode, which is Tauri's own line for its shortcuts. Our updater
; runs the installer with /S, and a silent install never runs the old
; uninstaller at all. The one uninstall that is really half a reinstall is the
; installer run by hand over an older copy: its reinstall page uninstalls
; first, without /UPDATE. Login start is written down before it goes so the
; POSTINSTALL above can have the new service put it back.
;
; Saves, settings and the local database stay, same as `install::remove`.
!macro NSIS_HOOK_POSTUNINSTALL
  StrCmp $UpdateMode "1" hoard_postun_done
  Push $0
  Push $1
  StrCpy $1 "0"
  ClearErrors
  ExecWait 'schtasks /Query /TN HoardSync' $0
  IfErrors hoard_postun_no_task
  StrCmp $0 "0" 0 hoard_postun_no_task
    StrCpy $1 "1"
  hoard_postun_no_task:
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "HoardSync"
  StrCmp $0 "" hoard_postun_no_run
    StrCpy $1 "1"
  hoard_postun_no_run:
  StrCmp $1 "1" 0 hoard_postun_unmarked
    CreateDirectory "$APPDATA\hoard\hoard\config"
    FileOpen $0 "$APPDATA\hoard\hoard\config\login-start-removed" w
    FileClose $0
  hoard_postun_unmarked:
  ExecWait 'schtasks /Delete /TN HoardSync /F'
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "HoardSync"
  Delete "$APPDATA\hoard\hoard\config\service-exec.txt"
  Delete "$APPDATA\hoard\hoard\config\install.json"
  Delete "$LOCALAPPDATA\hoard\bin\hoard.exe"
  Delete "$LOCALAPPDATA\hoard\bin\hoardd.exe"
  RMDir "$LOCALAPPDATA\hoard\bin"
  ; Only if it emptied: an install that predates the state move still keeps
  ; its local data under here.
  RMDir "$LOCALAPPDATA\hoard"
  Pop $1
  Pop $0
  hoard_postun_done:
!macroend
