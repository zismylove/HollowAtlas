from __future__ import annotations

import os
import queue
import threading
import tkinter as tk
from pathlib import Path
from tkinter import filedialog, messagebox, ttk

from hollowatlas.core.packer import pack_folder
from hollowatlas.core.scanner import scan_folder
from hollowatlas.core.types import PackResult, ScanResult
from hollowatlas.gui.atlas_preview import AtlasPreview
from hollowatlas.gui.file_tree import FileTree
from hollowatlas.gui.log_panel import LogPanel
from hollowatlas.gui.options_panel import OptionsPanel

try:
    from tkinterdnd2 import DND_FILES, TkinterDnD

    BaseTk = TkinterDnD.Tk
    HAS_DND = True
except Exception:  # tkinterdnd2 is optional; the file picker remains the fallback.
    DND_FILES = None
    BaseTk = tk.Tk
    HAS_DND = False


class HollowAtlasApp(BaseTk):
    def __init__(self):
        super().__init__()
        self.title("HollowAtlas")
        self.geometry("1180x760")
        self.minsize(980, 620)

        self.input_path: Path | None = None
        self.output_path: Path | None = None
        self.scan_result: ScanResult | None = None
        self.pack_result: PackResult | None = None
        self.events: "queue.Queue[tuple[str, object]]" = queue.Queue()

        self._build_ui()
        self.after(80, self._poll_events)

    def _build_ui(self) -> None:
        toolbar = ttk.Frame(self, padding=(8, 6))
        toolbar.grid(row=0, column=0, sticky="ew")
        toolbar.columnconfigure(5, weight=1)

        ttk.Button(toolbar, text="Input", command=self.choose_input).grid(row=0, column=0, padx=3)
        ttk.Button(toolbar, text="Output", command=self.choose_output).grid(row=0, column=1, padx=3)
        ttk.Button(toolbar, text="Pack", command=self.start_pack).grid(row=0, column=2, padx=3)
        ttk.Button(toolbar, text="Open Output", command=self.open_output).grid(row=0, column=3, padx=3)
        ttk.Button(toolbar, text="Clear", command=self.clear).grid(row=0, column=4, padx=3)
        self.path_label = ttk.Label(toolbar, text="No folder selected", anchor="w")
        self.path_label.grid(row=0, column=5, sticky="ew", padx=8)

        main = ttk.PanedWindow(self, orient="horizontal")
        main.grid(row=1, column=0, sticky="nsew")
        self.rowconfigure(1, weight=1)
        self.columnconfigure(0, weight=1)

        self.file_tree = FileTree(main)
        self.preview = AtlasPreview(main)
        self.options = OptionsPanel(main)
        main.add(self.file_tree, weight=1)
        main.add(self.preview, weight=4)
        main.add(self.options, weight=1)

        self.logs = LogPanel(self)
        self.logs.grid(row=2, column=0, sticky="ew")

        if HAS_DND and DND_FILES is not None:
            self.drop_target_register(DND_FILES)
            self.dnd_bind("<<Drop>>", self._on_drop)

    def choose_input(self) -> None:
        selected = filedialog.askdirectory(title="Select input folder")
        if not selected:
            return
        self.input_path = Path(selected)
        if self.output_path is None:
            self.output_path = self.input_path / "atlas_out"
        self._scan_selected()

    def _on_drop(self, event: tk.Event) -> None:
        paths = self.tk.splitlist(event.data)
        if not paths:
            return
        dropped = Path(paths[0])
        if dropped.is_file():
            dropped = dropped.parent
        if not dropped.is_dir():
            self.logs.add("warning", f"Drop ignored, not a folder: {dropped}")
            return
        self.input_path = dropped
        if self.output_path is None:
            self.output_path = self.input_path / "atlas_out"
        self._scan_selected()

    def choose_output(self) -> None:
        selected = filedialog.askdirectory(title="Select output folder")
        if not selected:
            return
        self.output_path = Path(selected)
        self._update_path_label()

    def _scan_selected(self) -> None:
        if self.input_path is None:
            return
        try:
            self.scan_result = scan_folder(self.input_path)
        except Exception as exc:
            messagebox.showerror("Scan failed", str(exc))
            self.logs.add("error", str(exc))
            return

        self.file_tree.load(self.scan_result)
        self.logs.add("info", f"Scanned: {self.input_path}")
        self.logs.add("info", f"Images found: {self.scan_result.total_images}")
        for warning in self.scan_result.warnings:
            self.logs.add("warning", warning)
        self._update_path_label()

    def start_pack(self) -> None:
        if self.input_path is None:
            messagebox.showwarning("Missing input", "Choose an input folder first.")
            return
        if self.output_path is None:
            self.output_path = self.input_path / "atlas_out"

        config = self.options.get_config()
        self.logs.add("info", "Pack started.")
        thread = threading.Thread(
            target=self._pack_worker,
            args=(self.input_path, self.output_path, config),
            daemon=True,
        )
        thread.start()

    def _pack_worker(self, input_path: Path, output_path: Path, config) -> None:
        try:
            result = pack_folder(input_path, output_path, config)
        except Exception as exc:
            self.events.put(("error", str(exc)))
            return
        self.events.put(("packed", result))

    def _poll_events(self) -> None:
        while True:
            try:
                event, payload = self.events.get_nowait()
            except queue.Empty:
                break

            if event == "error":
                self.logs.add("error", str(payload))
                messagebox.showerror("Pack failed", str(payload))
            elif event == "packed":
                result = payload
                self.pack_result = result
                for log in result.logs:
                    self.logs.add(log.level, log.message)
                if result.atlases:
                    self.preview.load(result.atlases[0], self.options.show_bounds.get())
                self._update_path_label()

        self.after(80, self._poll_events)

    def open_output(self) -> None:
        if self.output_path is None:
            messagebox.showwarning("Missing output", "Choose or create an output folder first.")
            return
        self.output_path.mkdir(parents=True, exist_ok=True)
        os.startfile(self.output_path)

    def clear(self) -> None:
        self.scan_result = None
        self.pack_result = None
        self.input_path = None
        self.file_tree.clear()
        self.preview.clear()
        self.logs.clear()
        self._update_path_label()

    def _update_path_label(self) -> None:
        input_text = str(self.input_path) if self.input_path else "No input"
        output_text = str(self.output_path) if self.output_path else "No output"
        self.path_label.configure(text=f"Input: {input_text} | Output: {output_text}")


def run_app() -> None:
    app = HollowAtlasApp()
    app.mainloop()


if __name__ == "__main__":
    run_app()
