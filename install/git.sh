configure_git_identity() {
  local name=${1:-}
  local email=${2:-}

  [[ -n $name ]] || die "git user name is required"
  [[ -n $email ]] || die "git user email is required"

  git config --global user.name "$name"
  git config --global user.email "$email"
  git config --global init.defaultBranch main
}
