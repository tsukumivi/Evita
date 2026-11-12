cask "evita" do
  version "1.1.0-nightly.20261112.13"
  sha256 "6cd4e919a5be50dbda7690349f3aa46e9570450799fbd2c3d442e051627c18fb"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
