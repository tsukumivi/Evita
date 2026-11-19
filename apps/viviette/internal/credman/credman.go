//go:build windows

package credman

import (
	"fmt"
	"syscall"
	"unsafe"
)

const credTypeGeneric = 1

var (
	modAdvapi32    = syscall.NewLazyDLL("advapi32.dll")
	procCredDelete = modAdvapi32.NewProc("CredDeleteW")
)

func targetName(key string) string { return "viviette:" + key }

func Delete(key string) error {
	target, err := syscall.UTF16PtrFromString(targetName(key))
	if err != nil {
		return err
	}
	r1, _, _ := procCredDelete.Call(uintptr(unsafe.Pointer(target)), credTypeGeneric, 0)
	if r1 == 0 {
		return fmt.Errorf("credman: CredDeleteW failed: %v", syscall.GetLastError())
	}
	return nil
}
