//go:build windows

package server

import (
	"encoding/json"
	"io"
	"net/http"
	"regexp"
	"strings"
	"sync"
	"time"

	"viviette/internal/config"
	"viviette/internal/presence"
)

// Discord application IDs are snowflakes: 17 to 20 digits.
var discordAppIDPattern = regexp.MustCompile(`^[0-9]{17,20}$`)

type stateResponse struct {
	presence.Status
	// Restored is set when startup replaced a broken config with the last-good copy.
	Restored *config.Restore `json:"config_restored,omitempty"`
}

// decodeStrict reads one small JSON object with no unknown fields and nothing after it.
func decodeStrict(w http.ResponseWriter, r *http.Request, limit int64, v any) bool {
	decoder := json.NewDecoder(http.MaxBytesReader(w, r.Body, limit))
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(v); err != nil {
		return false
	}
	return decoder.Decode(new(any)) == io.EOF
}

func writeJSON(w http.ResponseWriter, code int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	_ = json.NewEncoder(w).Encode(v)
}

// registerSetup adds the first-run checklist and test card endpoints.
func registerSetup(mux *http.ServeMux, d Deps, configMu *sync.Mutex, auth func(http.HandlerFunc) http.HandlerFunc) {
	defaultAppID := config.Defaults().Discord.ClientID
	setupBody := func() map[string]any {
		cfg := d.Cfg.Get()
		return map[string]any{"onboarded": cfg.UI.Onboarded, "client_id": cfg.Discord.ClientID, "default_client_id": cfg.Discord.ClientID == defaultAppID}
	}
	// update applies a change to the saved config under the panel's config lock.
	update := func(change func(*config.Config)) error {
		configMu.Lock()
		defer configMu.Unlock()
		current, err := config.Load(d.CfgPath)
		if err != nil {
			return err
		}
		change(current)
		if err := config.Save(current, d.CfgPath); err != nil {
			return err
		}
		_, err = d.Cfg.Reload()
		return err
	}

	mux.HandleFunc("/api/setup", auth(func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
		case http.MethodPost:
			var payload struct {
				Onboarded *bool   `json:"onboarded"`
				ClientID  *string `json:"client_id"`
			}
			if !decodeStrict(w, r, 256, &payload) || (payload.Onboarded == nil && payload.ClientID == nil) {
				http.Error(w, "invalid setup payload", 400)
				return
			}
			appID := ""
			if payload.ClientID != nil {
				// An empty ID goes back to Viviette's own Discord application.
				appID = strings.TrimSpace(*payload.ClientID)
				if appID == "" {
					appID = defaultAppID
				}
				if !discordAppIDPattern.MatchString(appID) {
					http.Error(w, "a Discord application ID is 17 to 20 digits", 400)
					return
				}
			}
			if err := update(func(c *config.Config) {
				if payload.Onboarded != nil {
					c.UI.Onboarded = *payload.Onboarded
				}
				if payload.ClientID != nil {
					c.Discord.ClientID = appID
				}
			}); err != nil {
				http.Error(w, err.Error(), 500)
				return
			}
		default:
			http.Error(w, "method", 405)
			return
		}
		writeJSON(w, 200, setupBody())
	}))

	mux.HandleFunc("/api/test-card", auth(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method", 405)
			return
		}
		if err := d.Engine.SendTestCard(); err != nil {
			http.Error(w, err.Error(), http.StatusConflict)
			return
		}
		writeJSON(w, http.StatusAccepted, map[string]any{"state": "sending", "seconds": int(presence.TestCardFor / time.Second)})
	}))

}
