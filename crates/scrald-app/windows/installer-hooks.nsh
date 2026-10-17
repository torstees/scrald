; Hooks for the Windows (NSIS) installer, wired up in tauri.conf.json
; (bundle > windows > nsis > installerHooks).
;
; File associations (DESIGN.md §11): Scrald is offered for .md, .markdown, and
; .mdown files in "Open with" and in Settings > Default apps, but never made
; the default. Tauri's own `fileAssociations` setting would overwrite each
; extension's default program, so it isn't used. Instead, each extension lists
; Scrald's program id under OpenWithProgids, which is the documented way to
; offer an app without taking over; Windows then asks the user.
;
; SHCTX is HKCU for a per-user install (the default) and HKLM for all users.

!define SCRALD_PROGID "Scrald.Markdown"
!define SCRALD_EXE "$INSTDIR\${MAINBINARYNAME}.exe"

!macro SCRALD_OFFER_EXTENSION EXT
  WriteRegStr SHCTX "Software\Classes\.${EXT}\OpenWithProgids" "${SCRALD_PROGID}" ""
  WriteRegStr SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe\SupportedTypes" ".${EXT}" ""
  WriteRegStr SHCTX "Software\Scrald\Capabilities\FileAssociations" ".${EXT}" "${SCRALD_PROGID}"
!macroend

!macro SCRALD_WITHDRAW_EXTENSION EXT
  DeleteRegValue SHCTX "Software\Classes\.${EXT}\OpenWithProgids" "${SCRALD_PROGID}"
  ; Remove the key only if nothing else is listed in it.
  DeleteRegKey /ifempty SHCTX "Software\Classes\.${EXT}\OpenWithProgids"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; The program id: how Scrald opens a Markdown file.
  WriteRegStr SHCTX "Software\Classes\${SCRALD_PROGID}" "" "Markdown document"
  WriteRegStr SHCTX "Software\Classes\${SCRALD_PROGID}\DefaultIcon" "" "${SCRALD_EXE},0"
  WriteRegStr SHCTX "Software\Classes\${SCRALD_PROGID}\shell\open\command" "" '"${SCRALD_EXE}" "%1"'

  ; The app's name and command in "Open with".
  WriteRegStr SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe" "FriendlyAppName" "Scrald"
  WriteRegStr SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe\shell\open\command" "" '"${SCRALD_EXE}" "%1"'

  ; Settings > Default apps lists the extensions Scrald can open.
  WriteRegStr SHCTX "Software\Scrald\Capabilities" "ApplicationName" "Scrald"
  WriteRegStr SHCTX "Software\Scrald\Capabilities" "ApplicationDescription" "A Markdown reader with light editing"
  WriteRegStr SHCTX "Software\RegisteredApplications" "Scrald" "Software\Scrald\Capabilities"

  !insertmacro SCRALD_OFFER_EXTENSION "md"
  !insertmacro SCRALD_OFFER_EXTENSION "markdown"
  !insertmacro SCRALD_OFFER_EXTENSION "mdown"

  ; Tell Explorer that associations changed (SHCNE_ASSOCCHANGED).
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro SCRALD_WITHDRAW_EXTENSION "md"
  !insertmacro SCRALD_WITHDRAW_EXTENSION "markdown"
  !insertmacro SCRALD_WITHDRAW_EXTENSION "mdown"
  DeleteRegValue SHCTX "Software\RegisteredApplications" "Scrald"
  DeleteRegKey SHCTX "Software\Scrald\Capabilities"
  DeleteRegKey /ifempty SHCTX "Software\Scrald"
  DeleteRegKey SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe"
  DeleteRegKey SHCTX "Software\Classes\${SCRALD_PROGID}"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend
