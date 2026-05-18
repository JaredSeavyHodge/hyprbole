Name = "hyprbole-apps"
NamePretty = "Hyprbole Apps"
HideFromProviderlist = true

function GetEntries()
  return {
    {
      Text = "Browser",
      Sub = "Brave Origin Nightly",
      Actions = {
        activate = "hyprbole-launch-browser",
      },
    },
    {
      Text = "Files",
      Sub = "Nautilus",
      Actions = {
        activate = "hyprbole-launch-files",
      },
    },
    {
      Text = "Passwords",
      Sub = "Open 1Password",
      Actions = {
        activate = "hyprbole-launch-passwords",
      },
    },
  }
end
