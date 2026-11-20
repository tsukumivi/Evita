cask "evita" do
  version "1.1.0-nightly.20261120.22"
  sha256 "fe52147fb451020688753ef976709bedde288ecf73f37eb480dc139337f08b3a"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
