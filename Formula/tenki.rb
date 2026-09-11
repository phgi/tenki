class Tenki < Formula
  desc "Animated terminal weather visualisation in pastel ASCII art"
  homepage "https://github.com/phgi/tenki"
  url "https://github.com/phgi/tenki/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "REPLACE_WITH_TARBALL_SHA256"
  license "MIT"
  head "https://github.com/phgi/tenki.git", branch: "main"

  # Rust is only needed to compile the formula. Homebrew installs it during
  # `brew install` and the finished binary has no runtime dependencies at all.
  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "tenki", shell_output("#{bin}/tenki --version")
    assert_match "Open-Meteo", shell_output("#{bin}/tenki --help")
  end
end
