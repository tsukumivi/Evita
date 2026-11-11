cask "evita" do
  version "1.1.0-nightly.20261111.11"
  sha256 "3d0656221cd4954811a46dee5b82bb42d380349983da317e7324965ab872939a"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
