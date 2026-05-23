from gi.repository import GObject, Nautilus

import os
import shutil
import subprocess
from urllib.parse import unquote, urlparse


IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp"}


class HyprboleVSCodeExtension(GObject.GObject, Nautilus.MenuProvider):
    def get_file_items(self, files):
        if len(files) != 1:
            return []

        file = files[0]
        path = self._file_path(file)
        if not path:
            return []

        items = []

        if shutil.which("code"):
            code_item = Nautilus.MenuItem(
                name="HyprboleVSCodeExtension::open_in_code",
                label="Open in VS Code",
                tip="Open the selected file or folder in Visual Studio Code",
            )
            code_item.connect("activate", self._open_in_code, path)
            items.append(code_item)

        if shutil.which("nvim"):
            nvim_item = Nautilus.MenuItem(
                name="HyprboleVSCodeExtension::open_in_neovim",
                label="Open in Neovim",
                tip="Open the selected file or folder in Neovim",
            )
            nvim_item.connect("activate", self._open_in_neovim, path)
            items.append(nvim_item)

        if self._is_wallpaper_candidate(path):
            wallpaper_item = Nautilus.MenuItem(
                name="HyprboleVSCodeExtension::set_as_wallpaper",
                label="Set as Wallpaper",
                tip="Set the selected image as the current wallpaper",
            )
            wallpaper_item.connect("activate", self._set_as_wallpaper, path)
            items.append(wallpaper_item)

        return items

    def _open_in_code(self, _menu, path):
        subprocess.Popen(["code", path])

    def _open_in_neovim(self, _menu, path):
        terminal_launcher = self._hyprbole_bin("hyprbole-launch-terminal")
        if terminal_launcher:
            subprocess.Popen([terminal_launcher, "nvim", path])
            return

        subprocess.Popen(["ghostty", "-e", "nvim", path])

    def _set_as_wallpaper(self, _menu, path):
        hyprbole_path = self._hyprbole_command()
        if not hyprbole_path:
            return

        env = os.environ.copy()
        env.setdefault("HYPRBOLE_PATH", os.path.expanduser("~/.local/share/hyprbole"))

        subprocess.Popen([hyprbole_path, "wallpaper", "set", path], env=env)

    def _is_wallpaper_candidate(self, path):
        if not os.path.isfile(path):
            return False

        return os.path.splitext(path)[1].lower() in IMAGE_EXTENSIONS

    def _hyprbole_command(self):
        local_bin = os.path.expanduser("~/.local/bin/hyprbole")
        if os.path.isfile(local_bin) and os.access(local_bin, os.X_OK):
            return local_bin

        hyprbole_root = os.environ.get("HYPRBOLE_PATH")
        if hyprbole_root:
            managed_bin = os.path.join(hyprbole_root, "bin", "hyprbole")
            if os.path.isfile(managed_bin) and os.access(managed_bin, os.X_OK):
                return managed_bin

        return shutil.which("hyprbole")

    def _hyprbole_bin(self, name):
        local_bin = os.path.expanduser(os.path.join("~/.local/bin", name))
        if os.path.isfile(local_bin) and os.access(local_bin, os.X_OK):
            return local_bin

        hyprbole_root = os.environ.get("HYPRBOLE_PATH")
        if not hyprbole_root:
            hyprbole_root = os.path.expanduser("~/.local/share/hyprbole")

        managed_bin = os.path.join(hyprbole_root, "bin", name)
        if os.path.isfile(managed_bin) and os.access(managed_bin, os.X_OK):
            return managed_bin

        return shutil.which(name)

    def _file_path(self, file):
        uri = file.get_uri()
        parsed = urlparse(uri)

        if parsed.scheme != "file":
            return None

        return os.path.normpath(unquote(parsed.path))
