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
  IfFileExists "$TEMP\hoard-restart-service.flag" 0 hoard_post_no_service
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
