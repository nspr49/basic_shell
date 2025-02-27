{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        #libPath = with pkgs; lib.makeLibraryPath [ libGL libxkbcommon wayland ];
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        rustVersion = "1.83.0";

      in {
        devShell = pkgs.mkShell rec {
          nativeBuildInputs = with pkgs; [ libxkbcommon libGL wayland ];

          buildInputs = with pkgs; [
            (pkgs.rust-bin.stable.${rustVersion}.default.override {
              extensions = [ "cargo" "clippy" "rustc" "rust-src" "rustfmt" ];
            })
            libxkbcommon
            wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            libGL
            vulkan-headers
            vulkan-loader
            vulkan-tools
            vulkan-tools-lunarg
            vulkan-extension-layer
            vulkan-validation-layers # don't need them *strictly* but immensely helpful

            pkgs.pkg-config

            pkgs.renderdoc
            pkgs.valgrind
            pkgs.kcachegrind
            pkgs.linuxPackages_latest.perf
          ];

          #LD_LIBRARY_PATH =
          #  "${pkgs.libxkbcommon}/lib:${pkgs.libGL}/lib:${pkgs.wayland}/lib";
          RUST_BACKTRACE = 1;
          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${
              builtins.toString (pkgs.lib.makeLibraryPath buildInputs)
            }";

          '';

          RUST_SRC_PATH =
            "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          #  "VK_LAYER_KHRONOS_validation:VK_EXT_swapchain_colorspace";
          #          VK_LAYER_PATH =
          #            "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
          #LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}"
          #          LD_LIBRARY_PATH =
          #            "${pkgs.xorg.libX11}/lib:${pkgs.xorg.libXcursor}/lib:${pkgs.xorg.libXrandr}/lib:${pkgs.xorg.libXi}/lib:${pkgs.vulkan-loader}/lib";
        };
      });
}
