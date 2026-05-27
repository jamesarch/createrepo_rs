{ lib, rustPlatform, fetchFromGitHub }:

rustPlatform.buildRustPackage rec {
  pname = "createrepo-rs";
  version = "0.1.8";

  src = fetchFromGitHub {
    owner = "jamesarch";
    repo = "createrepo_rs";
    rev = "v${version}";
    sha256 = "9f93784bf1d9504827c17009288f1f122e81a5975e651102405e0985be401f14";
  };

  cargoHash = ""; # nix-build -A createrepo-rs 2>&1 | grep cargoHash

  nativeBuildInputs = [ ];

  buildFeatures = [ ];

  meta = with lib; {
    description = "Pure Rust RPM repository metadata generator — dnf/yum-compatible, zero FFI";
    homepage = "https://github.com/jamesarch/createrepo_rs";
    changelog = "https://github.com/jamesarch/createrepo_rs/releases/tag/v${version}";
    license = licenses.gpl2Plus;
    mainProgram = "createrepo_rs";
    maintainers = with maintainers; [ ];
    platforms = platforms.linux ++ platforms.darwin;
  };
}
