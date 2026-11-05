; Участок E — хуки NSIS-установщика Boomly (bundle.windows.nsis.installerHooks).
;
; Данные пользователя лежат в %APPDATA%\Boomly (crate::paths::data_dir), а не в папке
; identifier, которую чистит стандартный деинсталлятор Tauri. Поэтому обновление и обычное
; удаление их не трогают. Удаляем их только если человек сам поставил галочку
; «Удалить данные приложения» и это не обновление ($UpdateMode).

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    RMDir /r "$APPDATA\Boomly"
  ${EndIf}
!macroend
