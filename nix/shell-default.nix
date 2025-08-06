{
  pkgs,
  rustToolchain,
  pythonBase,
  libsPath,
  systemPackages,
  setupScript,
  startScript,
  stopScript,
  statusScript,
}:

pkgs.mkShell {
  buildInputs = systemPackages ++ [
    pythonBase
    rustToolchain
    setupScript
    startScript
    stopScript
    statusScript
  ];
  shellHook = ''
    export LD_LIBRARY_PATH='${libsPath}':$LD_LIBRARY_PATH
    export FLUX_ROOT=$PWD
    export PYTHONPATH="''${FLUX_ROOT}/poc:$PYTHONPATH"
    export RUST_BACKTRACE=1
  '';
}
