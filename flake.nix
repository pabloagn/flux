{
  description = "Flux: A High-Performance Electrochemical-Processes Digital Twin";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };

        pythonEnv = pkgs.python311.withPackages (ps: with ps; [ uv ]);

        buildRustApp =
          { pname, src }:
          pkgs.rustPlatform.buildRustPackage {
            inherit pname src;
            version = "0.1.0";
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = with pkgs; [ pkg-config ];
            buildInputs = with pkgs; [ openssl ];
          };

        buildRustImage =
          { pname, bin }:
          pkgs.dockerTools.buildImage {
            name = pname;
            tag = "latest";
            copyToRoot = [ bin ];
            config = {
              Cmd = [ "/bin/${pname}" ];
            };
          };

        buildPythonImage =
          { pname, src }:
          let
            pythonApp = pkgs.python311.pkgs.buildPythonApplication {
              inherit pname src;
              version = "0.1.0";
              pyproject = true;
              nativeBuildInputs = [ pkgs.python311.pkgs.setuptools ];
            };
          in
          pkgs.dockerTools.buildImage {
            name = pname;
            tag = "latest";
            copyToRoot = [
              pkgs.bash
              pythonApp
            ];
            config = {
              Cmd = [ "${pythonApp}/bin/${pname}" ];
              WorkingDir = "/";
              Env = [ "PYTHONUNBUFFERED=1" ];
            };
          };

        buildNodejsImage =
          { pname, src }:
          let
            nodeApp = pkgs.build-npm-app {
              inherit pname src;
              version = "0.1.0";
              npmDepsHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="; # TODO: Replace with `nix-prefetch-npm-deps ./path/to/package-lock.json`
              buildPhase = ''
                runHook preBuild
                npm run build
                runHook postBuild
              '';
              installPhase = ''
                runHook preInstall
                mkdir -p $out/
                cp -r dist/* $out/
                runHook postInstall
              '';
            };
          in
          pkgs.dockerTools.buildImage {
            name = pname;
            tag = "latest";
            fromImage = pkgs.dockerTools.pullImage {
              imageName = "nginx";
              imageTag = "alpine";
              sha256 = "sha256-4O5283gC4zQfH0VSHB52Sw2y9O22x2aMCa1r1fIKsV4=";
            };
            copyToRoot = [ nodeApp ];
            config = {
              Cmd = [
                "nginx"
                "-g"
                "daemon off;"
              ];
            };
          };

      in
      {
        devShells = import ./nix {
          inherit pkgs rustToolchain;
          pythonBase = pythonEnv;
          libsPath = "${pkgs.stdenv.cc.cc.lib}/lib";
          systemPackages = with pkgs; [
            btop
            cmake
            confluent-platform
            curl
            dive
            docker-compose
            gcc
            git
            httpie
            jq
            just
            kcat
            lazydocker
            openssl
            pkg-config
            skopeo
            yq-go
          ];
        };

        packages = {
          questdb-with-healthcheck = pkgs.dockerTools.buildImage {
            name = "flux-questdb";
            tag = "7.3.10-custom";
            fromImage = pkgs.dockerTools.pullImage {
              imageName = "questdb/questdb";
              sha256 = "sha256-thn/LQLGkxqAPD/KumugBqj3N5AG7tzeFSHw0uYX95g=";
              imageDigest = "sha256:2a0408813dee86aa6e0d38f6d4411ea2918c6be34f45f3802f3a11f1e8000635";
            };
            runAsRoot = ''
              #!${pkgs.bash}/bin/bash
              ${pkgs.dockerTools.shadowSetup}
              apt-get update && apt-get install -y --no-install-recommends curl && rm -rf /var/lib/apt/lists/*
            '';
          };

          operator-tui-bin = buildRustApp {
            pname = "operator-tui";
            src = ./apps/operator-tui;
          };
          operator-tui = buildRustImage {
            pname = "operator-tui";
            bin = self.packages.${system}.operator-tui-bin;
          };

          audit-logger-bin = buildRustApp {
            pname = "audit-logger";
            src = ./services/audit-logger;
          };
          audit-logger = buildRustImage {
            pname = "audit-logger";
            bin = self.packages.${system}.audit-logger-bin;
          };

          control-system-bin = buildRustApp {
            pname = "control-system";
            src = ./services/control-system;
          };
          control-system = buildRustImage {
            pname = "control-system";
            bin = self.packages.${system}.control-system-bin;
          };

          safety-interlock-bin = buildRustApp {
            pname = "safety-interlock";
            src = ./services/safety-interlock;
          };
          safety-interlock = buildRustImage {
            pname = "safety-interlock";
            bin = self.packages.${system}.safety-interlock-bin;
          };

          state-manager-bin = buildRustApp {
            pname = "state-manager";
            src = ./services/state-manager;
          };

          state-manager = buildRustImage {
            pname = "state-manager";
            bin = self.packages.${system}.state-manager-bin;
          };

          alarm-manager = buildPythonImage {
            pname = "alarm-manager";
            src = ./services/alarm-manager;
          };
          data-pipeline = buildPythonImage {
            pname = "data-pipeline";
            src = ./services/data-pipeline;
          };
          historian = buildPythonImage {
            pname = "historian";
            src = ./services/historian;
          };
          kpi-engine = buildPythonImage {
            pname = "kpi-engine";
            src = ./services/kpi-engine;
          };
          ml-platform = buildPythonImage {
            pname = "ml-platform";
            src = ./services/ml-platform;
          };

          web-dashboard = buildNodejsImage {
            pname = "web-dashboard";
            src = ./apps/web-dashboard;
          };

          simulator = pkgs.stdenv.mkDerivation {
            pname = "flux-simulator";
            version = "0.1.0";
            src = ./services/simulator;
            nativeBuildInputs = [ pythonEnv ];
            installPhase = ''
              mkdir -p $out/bin
              cp -r flux $out/
              cat > $out/bin/flux-simulator <<EOF
              #!${pkgs.bash}/bin/bash
              export PYTHONPATH=$out:\$PYTHONPATH
              ${pythonEnv}/bin/python $out/flux/simulation/plant.py "\$@"
              EOF
              chmod +x $out/bin/flux-simulator
            '';
          };

          all-images = pkgs.buildEnv {
            name = "all-flux-images";
            paths = with self.packages.${system}; [
              questdb-with-healthcheck
              operator-tui
              audit-logger
              control-system
              safety-interlock
              state-manager
              alarm-manager
              data-pipeline
              historian
              kpi-engine
              ml-platform
              web-dashboard
            ];
          };
        };

        apps = {
          simulator = flake-utils.lib.mkApp { drv = self.packages.${system}.simulator; };
          tui = flake-utils.lib.mkApp { drv = self.packages.${system}.operator-tui-bin; };
          default = self.apps.${system}.tui;
        };
      }
    );
}
