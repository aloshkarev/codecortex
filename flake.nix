{
  description = "CodeCortex Nix build, check, and release definitions";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;

        commonBuildInputs =
          [
            pkgs.openssl
          ]
          ++ lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];

        commonNativeBuildInputs = [
          pkgs.pkg-config
          pkgs.cmake
          pkgs.protobuf
          pkgs.clang
        ];

        cortex = pkgs.rustPlatform.buildRustPackage {
          pname = "cortex";
          version = "1.0.1";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          cargoBuildFlags = [
            "-p"
            "cortex-cli"
          ];

          nativeBuildInputs = commonNativeBuildInputs;
          buildInputs = commonBuildInputs;

          # rsmgclient's macOS build script hardcodes package-manager probes
          # (`port`/`brew`) that are unavailable in a pure Nix build sandbox.
          preConfigure = lib.optionalString pkgs.stdenv.isDarwin ''
            mkdir -p .nix-fake-port/bin
            cat > .nix-fake-port/bin/port <<'EOF'
            #!/usr/bin/env sh
            if [ "$1" = "installed" ] && [ "$2" = "openssl" ]; then
              echo "  openssl @3.0.0_0 (active)"
              exit 0
            fi
            exit 0
            EOF
            chmod +x .nix-fake-port/bin/port

            mkdir -p .nix-fake-port/libexec/openssl3/lib
            ln -sf ${pkgs.openssl.out}/lib/libssl.dylib .nix-fake-port/libexec/openssl3/lib/libssl.dylib
            ln -sf ${pkgs.openssl.out}/lib/libcrypto.dylib .nix-fake-port/libexec/openssl3/lib/libcrypto.dylib
            export PATH="$PWD/.nix-fake-port/bin:$PATH"
          '';

          doCheck = false;

          installPhase = ''
            runHook preInstall
            CORTEX_BIN_PATH="$(echo target/*/release/cortex-cli | awk '{print $1}')"
            if [ ! -f "$CORTEX_BIN_PATH" ]; then
              CORTEX_BIN_PATH="target/release/cortex-cli"
            fi
            install -Dm755 "$CORTEX_BIN_PATH" "$out/bin/cortex"
            ln -s "$out/bin/cortex" "$out/bin/cortex-cli"
            runHook postInstall
          '';

          meta = with lib; {
            description = "CodeCortex CLI";
            homepage = "https://github.com/aloshkarev/codecortex";
            license = licenses.asl20;
            mainProgram = "cortex";
            platforms = platforms.unix;
          };
        };

        mkCheck =
          name: command:
          pkgs.runCommand name
            {
              nativeBuildInputs = commonNativeBuildInputs ++ [
                pkgs.cargo
                pkgs.rustc
                pkgs.rustfmt
                pkgs.clippy
                pkgs.cacert
              ];
              buildInputs = commonBuildInputs;
            }
            ''
              export HOME="$TMPDIR"
              export CARGO_HOME="$TMPDIR/cargo-home"
              export RUSTUP_HOME="$TMPDIR/rustup-home"
              export CARGO_TARGET_DIR="$TMPDIR/target"
              export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
              export NIX_SSL_CERT_FILE="$SSL_CERT_FILE"
              export CARGO_HTTP_CAINFO="$SSL_CERT_FILE"
              mkdir -p "$CARGO_HOME" "$RUSTUP_HOME"
              ${lib.optionalString pkgs.stdenv.isDarwin ''
                mkdir -p "$TMPDIR/.nix-fake-port/bin"
                cat > "$TMPDIR/.nix-fake-port/bin/port" <<'EOF'
                #!/usr/bin/env sh
                if [ "$1" = "installed" ] && [ "$2" = "openssl" ]; then
                  echo "  openssl @3.0.0_0 (active)"
                  exit 0
                fi
                exit 0
                EOF
                chmod +x "$TMPDIR/.nix-fake-port/bin/port"

                mkdir -p "$TMPDIR/.nix-fake-port/libexec/openssl3/lib"
                ln -sf ${pkgs.openssl.out}/lib/libssl.dylib "$TMPDIR/.nix-fake-port/libexec/openssl3/lib/libssl.dylib"
                ln -sf ${pkgs.openssl.out}/lib/libcrypto.dylib "$TMPDIR/.nix-fake-port/libexec/openssl3/lib/libcrypto.dylib"
                export PATH="$TMPDIR/.nix-fake-port/bin:$PATH"
              ''}
              cd ${./.}
              ${command}
              touch "$out"
            '';
      in
      {
        packages = {
          default = cortex;
          cortex = cortex;
        };

        apps = {
          default = {
            type = "app";
            program = "${cortex}/bin/cortex";
          };
          cortex = {
            type = "app";
            program = "${cortex}/bin/cortex";
          };
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = commonNativeBuildInputs ++ [
            pkgs.cargo
            pkgs.rustc
            pkgs.rustfmt
            pkgs.clippy
            pkgs.rust-analyzer
            pkgs.git
          ];

          buildInputs = commonBuildInputs;

          shellHook = ''
            export CARGO_TERM_COLOR=always
            echo "CodeCortex dev shell ready."
            echo "Run: cargo build, cargo test --workspace, cargo clippy --all-targets --all-features"
          '';
        };

        checks = {
          format = mkCheck "codecortex-fmt" "cargo fmt --all -- --check";
          lint =
            mkCheck "codecortex-clippy"
              "cargo clippy --workspace --all-targets --all-features -- -D warnings";
          tests = mkCheck "codecortex-tests" "cargo test --workspace --locked";
          mcpToolSurfaceGuard =
            mkCheck "codecortex-mcp-tool-surface-guard"
              "cargo test -p cortex-mcp --test tool_surface_matrix -- --nocapture";
          integrationFixtureGuard =
            mkCheck "codecortex-integration-fixture-guard"
              "cargo test -p cortex-cli --test language_matrix_integration fixtures_are_complete_and_pinned -- --nocapture";
        };
      }
    );
}
