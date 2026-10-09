{
  description = "ReliveVR protocol reverse-engineering and Linux server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f {
        pkgs = import nixpkgs { inherit system; };
      });
    in
    {
      devShells = forAllSystems ({ pkgs }: {
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
          shellHook = ''
            echo "ReliveVR RE shell ready"
          '';
        };
      });

      # packages.default left for later once Cargo.lock is present
    };
}
