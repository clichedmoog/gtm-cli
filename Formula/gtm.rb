class Gtm < Formula
  desc "Google Tag Manager CLI — built for humans and AI agents"
  homepage "https://github.com/clichedmoog/gtmcli"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/clichedmoog/gtmcli/releases/download/v#{version}/gtm-aarch64-apple-darwin.tar.gz"
      # sha256 will be filled after first release
    else
      url "https://github.com/clichedmoog/gtmcli/releases/download/v#{version}/gtm-x86_64-apple-darwin.tar.gz"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/clichedmoog/gtmcli/releases/download/v#{version}/gtm-aarch64-unknown-linux-gnu.tar.gz"
    else
      url "https://github.com/clichedmoog/gtmcli/releases/download/v#{version}/gtm-x86_64-unknown-linux-gnu.tar.gz"
    end
  end

  def install
    bin.install "gtm"
  end

  test do
    assert_match "gtm #{version}", shell_output("#{bin}/gtm --version")
  end
end
