{
  pkgs,
  rustToolchain,
  pythonBase,
  libsPath,
  systemPackages,
}:

pkgs.mkShell {
  name = "shell-default";

  buildInputs = systemPackages ++ [
    pythonBase
    rustToolchain
  ];
  shellHook = ''
    export LD_LIBRARY_PATH='${libsPath}':$LD_LIBRARY_PATH
    export FLUX_ROOT=$PWD
    export PYTHONPATH="''${FLUX_ROOT}/poc:$PYTHONPATH"
    export RUST_BACKTRACE=1
    export RUST_LOG=error
    export UV_LOG=error
    export UV_NO_PROGRESS=true
    export SETUPTOOLS_ENABLE_FEATURES=""
  '';
}
