cask "evita" do
  version "1.0.0"
  sha256 "f0a98e08480fe98f295b4fb441b653e181cacaa3f4ae59cd3a2ea12d275d3b21"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
