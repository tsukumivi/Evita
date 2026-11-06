@echo off
rem Runs Evita with its data beside this file. Security software can deny an unsigned build any
rem write inside the user profile (ERROR_ACCESS_DENIED), and then the cache, saved logins and the
rem diagnostic log all stay read-only. Put this file next to evita.exe and start Evita with it.
setlocal
set "ROOT=%~dp0"
if not exist "%ROOT%evita.exe" (
	echo Put this file next to evita.exe.
	exit /b 1
)
if not exist "%ROOT%data" mkdir "%ROOT%data"
set "EVITA_DATA_DIR=%ROOT%data"
start "" "%ROOT%evita.exe" %*
endlocal
