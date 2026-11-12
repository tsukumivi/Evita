// Package version holds the release version stamped in at build time.
package version

// Version is overwritten by the release build with
// -ldflags "-X viviette/internal/version.Version=<release-version>".
// Local and pull-request builds keep "dev".
var Version = "dev"
