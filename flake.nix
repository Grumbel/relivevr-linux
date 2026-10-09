{
  description = "ReliveVR protocol reverse-engineering and Linux server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f {
        inherit system;
        pkgs = import nixpkgs { inherit system; };
      });
    in
    {
      packages = forAllSystems ({ system, pkgs }: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "relivevr-server";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          meta = with pkgs.lib; {
            description = "Experimental ReliveVR (AMD Wireless GVR) protocol probe / server for Linux";
            license = licenses.gpl3Plus;
            platforms = platforms.linux;
            mainProgram = "relivevr-server";
          };
        };
      });

      devShells = forAllSystems ({ system, pkgs }: {
        default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            pkg-config
            openssl
            radare2
            python3
            python3Packages.scapy
            tshark
          ];
          inputsFrom = [ self.packages.${system}.default ];
          shellHook = ''
            echo "ReliveVR RE shell ready"
          '';
        };
      });

      apps = forAllSystems ({ system, pkgs }: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/relivevr-server";
        };
      });
    };
}
