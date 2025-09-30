{ pkgs, lib, config, inputs, ... }:

let
  pkgs-stable = import inputs.nixpkgs-stable { system = pkgs.stdenv.system; };
  pkgs-unstable = import inputs.nixpkgs-unstable { system = pkgs.stdenv.system; };
in
{
  env.GREET = "Rust Chat";

  packages = [
    pkgs-stable.git
    pkgs-stable.figlet
    pkgs-stable.lolcat
    pkgs-stable.watchman
    pkgs-stable.inotify-tools
  ];

  languages.rust = {
    enable = true;
    channel = "stable";
  };

  scripts.hello.exec = ''
    figlet -w 120 $GREET | lolcat
  '';

  enterShell = ''
    hello
  '';

}

