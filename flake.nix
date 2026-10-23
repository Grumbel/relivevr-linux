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

      # Loaded at runtime by winit/glutin (dlopen), not linked.
      glLibs = pkgs: with pkgs; [
        libGL libglvnd libx11 libxcursor libxi libxrandr
        libxkbcommon wayland
      ];

      # Headset helpers (Lenovo Mirage Solo over adb).
      headsetApps = pkgs:
        let
          app = name: description: text: {
            type = "app";
            meta.description = description;
            program = "${pkgs.writeShellApplication {
              inherit name text;
              runtimeInputs = [ pkgs.android-tools ];
            }}/bin/${name}";
          };
        in
        {
          # Restart the ReliveVR client so it reconnects to a fresh server.
          restart-client = app "relivevr-restart-client" "Restart the ReliveVR app on the headset (adb)" ''
            adb shell am force-stop com.amd.wirelessgvr
            sleep 1
            adb shell am start -n com.amd.wirelessgvr/.MainActivity
          '';
          # Client log (decoder health, pose lookups, configured FOV/codec).
          logcat = app "relivevr-logcat" "Follow the ReliveVR client log (adb logcat, AMF_TRACE)" ''
            adb logcat -c
            adb logcat -v time | grep --line-buffered AMF_TRACE
          '';
        };
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
          nativeBuildInputs = with pkgs; [ pkg-config makeWrapper ];
          buildInputs = with pkgs; [
            libGL libglvnd
            libx11 libxcursor libxi libxrandr libxcb
            libxkbcommon wayland
          ];
          # GL/X11 for the RELIVEVR_VIZ=1 window; hardware and libx264 encodes
          # pipe through the ffmpeg CLI.
          postFixup = ''
            wrapProgram $out/bin/relivevr-server \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath (glLibs pkgs)} \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.ffmpeg ]}
          '';
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
            ffmpeg
            # OpenGL visualizer
            libGL
            libglvnd
            libx11
            libxcursor
            libxi
            libxrandr
            libxcb
            libxkbcommon
            wayland
          ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (glLibs pkgs);
          inputsFrom = [ self.packages.${system}.default ];
          shellHook = ''
            echo "ReliveVR RE shell ready"
          '';
        };
      });

      apps = forAllSystems ({ system, pkgs }:
        let server = self.packages.${system}.default; in
        {
          # Headless server: test pattern / static stereo stream.
          default = {
            type = "app";
            meta.description = "ReliveVR server (test stream)";
            program = "${server}/bin/relivevr-server";
          };
          # Server with the OpenGL pose visualizer, streamed live to the headset.
          viz = {
            type = "app";
            meta.description = "ReliveVR server with the OpenGL visualizer as live stream";
            program = "${pkgs.writeShellScript "relivevr-viz" ''
              export RELIVEVR_VIZ=1
              exec ${server}/bin/relivevr-server "$@"
            ''}";
          };
        } // headsetApps pkgs);
    };
}
