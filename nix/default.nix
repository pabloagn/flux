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

{
  default = import ./shell-default.nix {
    inherit
      pkgs
      rustToolchain
      pythonBase
      libsPath
      systemPackages
      setupScript
      startScript
      stopScript
      statusScript
      ;
  };

  poc = import ./shell-poc.nix {
    inherit
      pkgs
      pythonBase
      libsPath
      systemPackages
      setupScript
      startScript
      stopScript
      statusScript
      ;
  };

  core = import ./shell-core.nix {
    inherit
      pkgs
      rustToolchain
      libsPath
      systemPackages
      startScript
      stopScript
      ;
  };
}
