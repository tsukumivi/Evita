cask "evita" do
  version "1.1.0-nightly.20261113.15"
  sha256 "292673b9ff8cd5e24e038047ad5b286b959ad70c304697c82a8b32610df6c9be"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
