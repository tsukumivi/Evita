cask "evita" do
  version "1.1.0-nightly.20261124.26"
  sha256 "b8c0e673b7f1eb6c3191ca956fa142452a0b49f8ec351fd73b901446d9c6647c"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
