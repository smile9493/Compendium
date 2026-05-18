{
  description = "rsut-pdf-mcp — Rust PDF extraction microservices and CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable."1.91.0".default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
            "rustfmt"
            "llvm-tools"
          ];
          targets = [
            "wasm32-unknown-unknown"
          ];
        };

        buildInputs = with pkgs; [
          openssl
          pkg-config
          clang
        ];

        nativeBuildInputs = with pkgs; [
          rustToolchain
          cargo-deny
          cargo-audit
          cargo-nextest
          just
          typos
          taplo
          nodejs_20
          python312
          python312Packages.pre-commit
        ] ++ lib.optionals stdenv.isDarwin [
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.CoreFoundation
          darwin.apple_sdk.frameworks.SystemConfiguration
          iconv
        ] ++ lib.optionals stdenv.isLinux [
          libayatana-appindicator
        ];

      in
      {
        # ── Development shells ─────────────────────────────────

        devShells = {
          default = pkgs.mkShell {
            name = "rsut-pdf-mcp-dev";
            inherit buildInputs nativeBuildInputs;

            shellHook = ''
              export RUST_BACKTRACE=1
              export RUST_LOG=debug

              echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
              echo "  rsut-pdf-mcp development environment"
              echo "  Rust:  $(rustc --version)"
              echo "  Cargo: $(cargo --version)"
              echo ""
              echo "  Commands:"
              echo "    just              Show all tasks"
              echo "    just ci           Run full CI checks"
              echo "    just build        Build workspace"
              echo "    just test         Run all tests"
              echo ""
              echo "  PDFium note:"
              echo "    Set PDFIUM_LIB_PATH=/path/to/libpdfium.so"
              echo "    or download binaries from:"
              echo "    https://github.com/bblanchon/pdfium-binaries/releases"
              echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            '';

            RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
            CARGO_BUILD_TARGET_DIR = "target-nix";
          };

          ci = pkgs.mkShell {
            name = "rsut-pdf-mcp-ci";
            inherit buildInputs;

            nativeBuildInputs = with pkgs; [
              rustToolchain
              cargo-deny
              cargo-audit
              just
              typos
              taplo
            ];

            shellHook = ''
              export RUST_BACKTRACE=1
              export CARGO_BUILD_TARGET_DIR="target-ci"
            '';
          };
        };

        # ── Packages ──────────────────────────────────────────

        packages = {
          default = self.packages.${system}.pdf-mcp;

          pdf-mcp = pkgs.rustPlatform.buildRustPackage {
            pname = "pdf-mcp";
            version = "0.3.0";
            src = ./pdf-module-rs;
            cargoLock.lockFile = ./pdf-module-rs/Cargo.lock;

            nativeBuildInputs = with pkgs; [ pkg-config clang ];
            buildInputs = with pkgs; [ openssl ];

            buildAndTestSubdir = "crates/pdf-mcp";

            meta = with pkgs.lib; {
              description = "MCP stdio pipe for PDF extraction via pdfium";
              license = licenses.mit;
              mainProgram = "pdf-mcp";
            };
          };

          pdf-web = pkgs.rustPlatform.buildRustPackage {
            pname = "pdf-web";
            version = "0.1.0";
            src = ./pdf-module-rs;
            cargoLock.lockFile = ./pdf-module-rs/Cargo.lock;

            nativeBuildInputs = with pkgs; [ pkg-config clang ];
            buildInputs = with pkgs; [ openssl ];

            buildAndTestSubdir = "crates/pdf-web";

            meta = with pkgs.lib; {
              description = "Lightweight embedded web panel for knowledge base management";
              license = licenses.mit;
              mainProgram = "pdf-web";
            };
          };

          pdf-cli = pkgs.rustPlatform.buildRustPackage {
            pname = "pdf-cli";
            version = "0.1.0";
            src = ./pdf-module-rs;
            cargoLock.lockFile = ./pdf-module-rs/Cargo.lock;

            nativeBuildInputs = with pkgs; [ pkg-config clang ];
            buildInputs = with pkgs; [ openssl ];

            buildAndTestSubdir = "crates/pdf-cli";

            meta = with pkgs.lib; {
              description = "CLI tool for PDF extraction and knowledge compilation";
              license = licenses.mit;
              mainProgram = "pdf-cli";
            };
          };
        };

        # ── Formatter ─────────────────────────────────────────

        formatter = pkgs.nixpkgs-fmt;
      }
    );
}