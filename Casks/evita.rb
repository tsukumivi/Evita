cask "evita" do
  version "1.1.0-nightly.20261109.7"
  sha256 "eb230d1781f856d710e301abb16629482b0e108ea0bb7a0e00b3d3eade3a7344"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
