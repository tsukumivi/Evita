//go:build windows

package main

import (
	"encoding/binary"
	"os"
	"path/filepath"
	"testing"
)

func TestBundledVivietteTrayIconValidICO(t *testing.T) {
	icon, err := os.ReadFile(filepath.Join("..", "..", "internal", "tray", "viviette-tray.ico"))
	if err != nil {
		t.Fatal(err)
	}
	if len(icon) < 22 || binary.LittleEndian.Uint16(icon[0:2]) != 0 || binary.LittleEndian.Uint16(icon[2:4]) != 1 {
		t.Fatal("bundled Viviette tray icon has an invalid ICO header")
	}
	count := int(binary.LittleEndian.Uint16(icon[4:6]))
	if count == 0 || 6+16*count > len(icon) {
		t.Fatal("bundled Viviette tray icon has an invalid image table")
	}
	for i := 0; i < count; i++ {
		entry := 6 + i*16
		length := int(binary.LittleEndian.Uint32(icon[entry+8 : entry+12]))
		offset := int(binary.LittleEndian.Uint32(icon[entry+12 : entry+16]))
		if length == 0 || offset < 6+16*count || offset+length > len(icon) {
			t.Fatalf("bundled Viviette icon image %d is out of bounds", i)
		}
	}
}
