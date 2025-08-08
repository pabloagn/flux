{
  pkgs,
  pythonBase,
  libsPath,
  systemPackages,
  setupScript,
  startScript,
  stopScript,
  statusScript,
}:

pkgs.mkShell {
  buildInputs =
    [
      pythonBase
      pkgs.uv
    ]
    ++ systemPackages
    ++ [
      setupScript
      startScript
      stopScript
      statusScript
    ];
  shellHook = ''
    export LD_LIBRARY_PATH='${libsPath}':$LD_LIBRARY_PATH
    export UV_LOG=error
    export RUST_LOG=error
    export UV_NO_PROGRESS=true
    export SETUPTOOLS_ENABLE_FEATURES=""
    [ -d .venv ] || uv venv
    source .venv/bin/activate
    export PYTHONPATH=$PWD:$PYTHONPATH
    echo "POC shell (Python) ready."
  '';
}
