class Chatter < Formula
  desc "Text-to-speech from the terminal, powered by Qwen3-TTS"
  homepage "https://github.com/isa/chatter"
  license "Apache-2.0"
  version "1.1.2"

  url "https://github.com/isa/chatter/archive/refs/tags/v1.1.2.tar.gz"
  sha256 "bc0b34f02f448279090f662879a99570d71fa5bc33338732488beb13ae7dff3b"

  depends_on "rust" => :build
  depends_on "python@3.12"

  def python3
    "python3.12"
  end

  def install
    # Tell PyO3 to link against Homebrew's Python 3.12
    ENV["PYO3_PYTHON"] = Formula["python@3.12"].opt_bin/python3
    ENV["PIP_DISABLE_PIP_VERSION_CHECK"] = "1"
    ENV["PIP_PROGRESS_BAR"] = "off"

    # Build the Rust binary (build.rs handles rpath for libpython)
    system "cargo", "install", *std_cargo_args

    # Create the bundled venv at libexec/venv/ — this is where the
    # binary looks for Python packages (../libexec/venv/ relative to bin/)
    venv = libexec/"venv"
    system Formula["python@3.12"].opt_bin/python3, "-m", "venv", venv

    # Determine which TTS backend to install based on platform
    pip = venv/"bin/pip"
    used_runtime_bundle = false

    # Optional fast-path: prebuilt runtime bundle.
    # Maintainers can publish a tar.gz containing a preconfigured `venv/`
    # tree and provide its URL via CHATTER_RUNTIME_BUNDLE_URL at build time.
    runtime_bundle_url = ENV["CHATTER_RUNTIME_BUNDLE_URL"]
    if runtime_bundle_url && !runtime_bundle_url.empty?
      runtime_bundle = buildpath/"chatter-runtime-venv.tar.gz"
      if system "curl", "-fL", runtime_bundle_url, "-o", runtime_bundle
        system "tar", "-xzf", runtime_bundle, "-C", libexec
        used_runtime_bundle = (libexec/"venv/bin/python").exist?
      end
    end

    unless used_runtime_bundle
      if Hardware::CPU.arm? && OS.mac?
        # Apple Silicon: use mlx-audio for optimized Metal inference (pinned deps)
        system pip, "install", "--no-cache-dir", "--quiet", "--only-binary", ":all:", "-r", buildpath/"requirements-mlx.txt"
      else
        # CUDA or CPU fallback: use qwen-tts
        system pip, "install", "--no-cache-dir", "--quiet", "qwen-tts"
      end
    end

    # Install the bridge package into the venv's site-packages.
    # The binary also does this at runtime (ensure_bridge_installed),
    # but pre-installing avoids a first-run write to the Cellar.
    site_packages = (venv/"lib").glob("python*/site-packages").first
    cp_r "chatter_bridge", site_packages/"chatter_bridge"
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
