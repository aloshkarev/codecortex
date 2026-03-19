let
  flake = builtins.getFlake (toString ./.);
  system = builtins.currentSystem;
in
flake.packages.${system}.default
