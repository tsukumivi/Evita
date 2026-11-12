cask "evita" do
  version "1.1.0-nightly.20261112.12"
  sha256 "16dff481815ccfba32f6a7c02fba631a04e90367cd542ee0c1b6b56d0a84796e"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
