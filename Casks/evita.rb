cask "evita" do
  version "1.0.1-nightly.20261109.6"
  sha256 "1014bbfdda3e13de7463e75764e695eae5604780bca6fcf1baca21a8dbe50167"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
