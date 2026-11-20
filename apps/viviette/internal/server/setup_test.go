//go:build windows

package server

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"viviette/internal/config"
	"viviette/internal/presence"
)

type setupFixture struct {
	h   http.Handler
	cfg *config.Holder
}

func newSetupFixture(t *testing.T) *setupFixture {
	t.Helper()
	path := filepath.Join(t.TempDir(), "setup.toml")
	if err := config.Save(config.Defaults(), path); err != nil {
		t.Fatal(err)
	}
	holder, err := config.NewHolder(path)
	if err != nil {
		t.Fatal(err)
	}
	f := &setupFixture{cfg: holder}
	f.h = Handler(Deps{
		Engine: presence.New(holder, nil, nil, ""), Cfg: holder, CfgPath: path, Token: "fixture-token",
	})
	return f
}

func decodeBody(t *testing.T, w *httptest.ResponseRecorder) map[string]any {
	t.Helper()
	var out map[string]any
	if err := json.Unmarshal(w.Body.Bytes(), &out); err != nil {
		t.Fatalf("body %q: %v", w.Body.String(), err)
	}
	return out
}

func TestSetupOnboardedAndDiscordAppID(t *testing.T) {
	f := newSetupFixture(t)
	if w := request(f.h, "GET", "/api/setup", "", false); w.Code != 401 {
		t.Fatalf("unauthenticated setup = %d", w.Code)
	}
	got := decodeBody(t, request(f.h, "GET", "/api/setup", "", true))
	if got["onboarded"] != false || got["default_client_id"] != true {
		t.Fatalf("initial setup = %v", got)
	}
	for _, body := range []string{`{}`, `{"onboarded":"yes"}`, `{"onboarded":true,"x":1}`, `{"onboarded":true} {}`, `{"client_id":"123"}`, `{"client_id":"12345678901234567a"}`, `{"client_id":"123456789012345678901"}`} {
		if w := request(f.h, "POST", "/api/setup", body, true); w.Code != 400 {
			t.Errorf("accepted %s: %d", body, w.Code)
		}
	}
	if f.cfg.Get().UI.Onboarded {
		t.Fatal("a rejected body changed the config")
	}
	got = decodeBody(t, request(f.h, "POST", "/api/setup", `{"onboarded":true}`, true))
	if got["onboarded"] != true || !f.cfg.Get().UI.Onboarded {
		t.Fatalf("onboarded not saved: %v", got)
	}
	loaded, err := config.Load(f.cfg.Path())
	if err != nil || !loaded.UI.Onboarded {
		t.Fatalf("onboarded not in the file: %v", err)
	}
	got = decodeBody(t, request(f.h, "POST", "/api/setup", `{"client_id":" 112233445566778899 "}`, true))
	if got["client_id"] != "112233445566778899" || got["default_client_id"] != false {
		t.Fatalf("custom app ID = %v", got)
	}
	got = decodeBody(t, request(f.h, "POST", "/api/setup", `{"client_id":""}`, true))
	if got["default_client_id"] != true || f.cfg.Get().Discord.ClientID != config.Defaults().Discord.ClientID {
		t.Fatalf("empty app ID did not restore Viviette's: %v", got)
	}
}

func TestTestCardEndpointQueuesOnce(t *testing.T) {
	f := newSetupFixture(t)
	if w := request(f.h, "POST", "/api/test-card", "", false); w.Code != 401 {
		t.Fatalf("unauthenticated test card = %d", w.Code)
	}
	if w := request(f.h, "GET", "/api/test-card", "", true); w.Code != 405 {
		t.Fatalf("GET test card = %d", w.Code)
	}
	if w := request(f.h, "POST", "/api/test-card", "", true); w.Code != http.StatusAccepted {
		t.Fatalf("test card = %d", w.Code)
	}
	if w := request(f.h, "POST", "/api/test-card", "", true); w.Code != http.StatusConflict {
		t.Fatalf("second test card = %d", w.Code)
	}
	state := decodeBody(t, request(f.h, "GET", "/api/state", "", true))
	if card, _ := state["test_card"].(map[string]any); card["state"] != "sending" {
		t.Fatalf("state test_card = %v", state["test_card"])
	}
}

// /api/state names the broken file only after a startup that fell back to the last-good copy.
func TestStateReportsRestoredConfig(t *testing.T) {
	f := newSetupFixture(t)
	if _, ok := decodeBody(t, request(f.h, "GET", "/api/state", "", true))["config_restored"]; ok {
		t.Fatal("a normal start reported a restored config")
	}

	path := f.cfg.Path()
	if err := os.WriteFile(path, []byte("[ui\ntheme = "), 0o644); err != nil {
		t.Fatal(err)
	}
	holder, err := config.NewHolder(path)
	if err != nil {
		t.Fatal(err)
	}
	h := Handler(Deps{Engine: presence.New(holder, nil, nil, ""), Cfg: holder, CfgPath: path, Token: "fixture-token"})
	got, _ := decodeBody(t, request(h, "GET", "/api/state", "", true))["config_restored"].(map[string]any)
	broken, _ := got["broken_path"].(string)
	if !strings.HasPrefix(filepath.Base(broken), "setup.toml.broken-") || got["error"] == "" || got["at"] == "" {
		t.Fatalf("config_restored = %v", got)
	}
	if _, err := os.Stat(broken); err != nil {
		t.Fatalf("broken file not kept: %v", err)
	}
}
