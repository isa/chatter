class Chatter < Formula
  desc "Text-to-speech from the terminal, powered by Qwen3-TTS"
  homepage "https://github.com/isa/chatter"
  license "MIT"
  version "1.0.0"

  url "https://github.com/isa/chatter/archive/refs/tags/v#{version}.tar.gz"
  sha256 "1e8ec734d6f4d743fb746256c5958dc87f8b5487ab0a851b030d26428a46cb6d"

  depends_on "rust" => :build
  depends_on "python@3.12"

  def python3
    "python3.12"
  end

  def install
    # Tell PyO3 to link against Homebrew's Python 3.12
    ENV["PYO3_PYTHON"] = Formula["python@3.12"].opt_bin/python3

    # Build the Rust binary (build.rs handles rpath for libpython)
    system "cargo", "install", *std_cargo_args

    # Create the bundled venv at libexec/venv/ — this is where the
    # binary looks for Python packages (../libexec/venv/ relative to bin/)
    venv = libexec/"venv"
    system Formula["python@3.12"].opt_bin/python3, "-m", "venv", venv

    # Determine which TTS backend to install based on platform
    pip = venv/"bin/pip"
    if Hardware::CPU.arm? && OS.mac?
      # Apple Silicon: use mlx-audio for optimized Metal inference (pinned deps)
      system pip, "install", "--no-cache-dir", "--quiet", "--only-binary", ":all:", "-r", buildpath/"requirements-mlx.txt"
    else
      # CUDA or CPU fallback: use qwen-tts
      system pip, "install", "--no-cache-dir", "qwen-tts"
    end

    # Install the bridge module into the venv's site-packages.
    # The binary also does this at runtime (ensure_bridge_installed),
    # but pre-installing avoids a first-run write to the Cellar.
    site_packages = (venv/"lib").glob("python*/site-packages").first
    cp "chatter_bridge.py", site_packages/"chatter_bridge.py"
  end

  def caveats
    <<~EOS
      After installing, download the TTS models (8bit by default, ~6 GB):

        chatter model download

      For higher quality (bf16, ~12 GB):

        chatter model download --variant bf16

      Run `chatter doctor` to verify your setup.
    EOS
  end

  test do
    # Verify the binary runs and doctor finds the bundled venv
    assert_match "Chatter Environment Check", shell_output("#{bin}/chatter doctor 2>&1")
  end
end
