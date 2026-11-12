//go:build windows

package sysutil

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"time"
	"unsafe"
)

var (
	modUser32                    = syscall.NewLazyDLL("user32.dll")
	procFindWindowW              = modUser32.NewProc("FindWindowW")
	procFindWindowExW            = modUser32.NewProc("FindWindowExW")
	procGetPropW                 = modUser32.NewProc("GetPropW")
	procPostMessageW             = modUser32.NewProc("PostMessageW")
	procGetWindowThreadProcessId = modUser32.NewProc("GetWindowThreadProcessId")
)

// These mirror internal/tray (trayClassName, cmdQuit); quit_test.go checks
// they still match. Posting the tray's own Quit command runs exactly what
// "Quit Viviette" does: stop the daemon, close the panel, remove the icon.
const (
	trayHostClass   = "VivietteTrayHost"
	panelEntryClass = "Static"
	panelEntryTitle = "VivietteControlPanelEntry"
	panelHWNDProp   = "ViviettePanelHWND"
	hwndMessage     = ^uintptr(2)
	trayCmdQuit     = 1002
	wmCommand       = 0x0111
	wmClose         = 0x0010
)

const stopPollInterval = 100 * time.Millisecond

// StopRunning asks a running Viviette started from this executable to quit
// the way the tray menu does, then waits up to wait for it to exit. A daemon
// running without a tray (autostart starts only `serve`) is stopped with
// StopDaemon, which is also what the tray's Quit uses. It returns nil when
// nothing of this install is left running.
func StopRunning(dashboardJSONPath string, wait time.Duration) error {
	exe, err := os.Executable()
	if err != nil {
		return err
	}
	return stopRunning(dashboardJSONPath, exe, trayHostClass, wait)
}

func stopRunning(dashboardJSONPath, exe, trayClass string, wait time.Duration) error {
	deadline := time.Now().Add(wait)

	panelPID := panelProcessPID(exe)
	if panelPID != 0 {
		if hwnd := findPanelWindow(exe); hwnd != 0 {
			procPostMessageW.Call(hwnd, wmClose, 0, 0)
		}
	}
	daemonPID := 0
	if hwnd := findWindowByClass(trayClass); hwnd != 0 {
		var pid uint32
		procGetWindowThreadProcessId.Call(hwnd, uintptr(unsafe.Pointer(&pid)))
		if pid != 0 && samePath(pidImageName(int(pid)), exe) {
			daemonPID = int(pid)
			procPostMessageW.Call(hwnd, wmCommand, trayCmdQuit, 0)
		}
	}
	panelGone := func() bool { return panelPID == 0 || !pidAlive(panelPID) }
	daemonGone := func() bool { return daemonPID == 0 || !pidAlive(daemonPID) }
	gone := func() bool { return panelGone() && daemonGone() && !daemonRunning(dashboardJSONPath, exe) }
	// The tray's Quit stops the daemon before it closes the panel, so the
	// panel's exit is the signal to wait for.
	for !panelGone() && time.Now().Before(deadline) {
		time.Sleep(stopPollInterval)
	}
	if daemonRunning(dashboardJSONPath, exe) {
		_ = StopDaemon(dashboardJSONPath, exe)
		for !gone() && time.Now().Before(deadline.Add(2*time.Second)) {
			time.Sleep(stopPollInterval)
		}
	}
	if !gone() {
		return errors.New("sysutil: Viviette is still running")
	}
	return nil
}

func findPanelWindow(exe string) uintptr {
	class, err := syscall.UTF16PtrFromString(panelEntryClass)
	if err != nil {
		return 0
	}
	title, err := syscall.UTF16PtrFromString(panelEntryTitle)
	if err != nil {
		return 0
	}
	entry, _, _ := procFindWindowExW.Call(hwndMessage, 0,
		uintptr(unsafe.Pointer(class)), uintptr(unsafe.Pointer(title)))
	if entry == 0 {
		return 0
	}
	property, err := syscall.UTF16PtrFromString(panelHWNDProp)
	if err != nil {
		return 0
	}
	panel, _, _ := procGetPropW.Call(entry, uintptr(unsafe.Pointer(property)))
	if panel == 0 {
		return 0
	}
	var pid uint32
	procGetWindowThreadProcessId.Call(panel, uintptr(unsafe.Pointer(&pid)))
	if pid == 0 || !samePath(pidImageName(int(pid)), exe) {
		return 0
	}
	return panel
}

func panelProcessPID(exe string) int {
	panel := findPanelWindow(exe)
	if panel == 0 {
		return 0
	}
	var pid uint32
	procGetWindowThreadProcessId.Call(panel, uintptr(unsafe.Pointer(&pid)))
	return int(pid)
}

func findWindowByClass(name string) uintptr {
	class, err := syscall.UTF16PtrFromString(name)
	if err != nil {
		return 0
	}
	h, _, _ := procFindWindowW.Call(uintptr(unsafe.Pointer(class)), 0)
	return h
}

// daemonRunning reports whether the PID in the dashboard endpoint is alive and
// is this executable, so a recycled PID is never mistaken for the daemon.
func daemonRunning(dashboardJSONPath, exe string) bool {
	pid, err := endpointPID(dashboardJSONPath)
	if err != nil || !pidAlive(pid) {
		return false
	}
	name := pidImageName(pid)
	if name == "" {
		return false
	}
	guard, err := filepath.Abs(exe)
	if err != nil {
		return false
	}
	return strings.EqualFold(filepath.Clean(name), filepath.Clean(guard))
}
