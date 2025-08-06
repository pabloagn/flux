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
    [ -d .venv ] || uv venv
    source .venv/bin/activate
    export PYTHONPATH=$PWD:$PYTHONPATH
    echo "POC shell (Python) ready."
  '';
}
