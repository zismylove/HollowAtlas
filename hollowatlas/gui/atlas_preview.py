from __future__ import annotations

import tkinter as tk
from tkinter import ttk
from typing import Optional

from PIL import Image, ImageTk

from hollowatlas.core.types import AtlasResult


class AtlasPreview(ttk.LabelFrame):
    def __init__(self, master: tk.Misc):
        super().__init__(master, text="Atlas Preview")
        toolbar = ttk.Frame(self)
        toolbar.grid(row=0, column=0, sticky="ew")

        ttk.Button(toolbar, text="-", width=3, command=lambda: self.zoom_by(0.8)).pack(side="left", padx=2, pady=2)
        ttk.Button(toolbar, text="+", width=3, command=lambda: self.zoom_by(1.25)).pack(side="left", padx=2, pady=2)
        ttk.Button(toolbar, text="Fit", command=self.fit).pack(side="left", padx=2, pady=2)
        self.info = ttk.Label(toolbar, text="No atlas")
        self.info.pack(side="left", padx=8)

        self.canvas = tk.Canvas(self, background="#f6f7f9", highlightthickness=0)
        self.canvas.grid(row=1, column=0, sticky="nsew")
        self.columnconfigure(0, weight=1)
        self.rowconfigure(1, weight=1)

        self.original: Optional[Image.Image] = None
        self.tk_image: Optional[ImageTk.PhotoImage] = None
        self.atlas: Optional[AtlasResult] = None
        self.zoom = 1.0
        self.offset_x = 20
        self.offset_y = 20
        self.show_bounds = True
        self._drag_start: Optional[tuple[int, int]] = None

        self.canvas.bind("<Configure>", lambda _event: self.redraw())
        self.canvas.bind("<MouseWheel>", self._on_mouse_wheel)
        self.canvas.bind("<ButtonPress-1>", self._on_drag_start)
        self.canvas.bind("<B1-Motion>", self._on_drag_move)

    def load(self, atlas: AtlasResult, show_bounds: bool = True) -> None:
        self.atlas = atlas
        self.original = Image.open(atlas.image_path).convert("RGBA")
        self.show_bounds = show_bounds
        self.zoom = 1.0
        self.offset_x = 20
        self.offset_y = 20
        self.info.configure(text=f"{atlas.width}x{atlas.height} | usage {atlas.usage * 100:.1f}%")
        self.fit()

    def clear(self) -> None:
        self.original = None
        self.tk_image = None
        self.atlas = None
        self.canvas.delete("all")
        self.info.configure(text="No atlas")

    def zoom_by(self, factor: float) -> None:
        self.zoom = max(0.05, min(16.0, self.zoom * factor))
        self.redraw()

    def fit(self) -> None:
        if self.original is None:
            return
        canvas_w = max(1, self.canvas.winfo_width())
        canvas_h = max(1, self.canvas.winfo_height())
        fit_zoom = min(canvas_w / self.original.width, canvas_h / self.original.height, 1.0) * 0.92
        self.zoom = max(0.05, fit_zoom)
        self.offset_x = int((canvas_w - self.original.width * self.zoom) / 2)
        self.offset_y = int((canvas_h - self.original.height * self.zoom) / 2)
        self.redraw()

    def redraw(self) -> None:
        self.canvas.delete("all")
        if self.original is None:
            return

        width = max(1, int(self.original.width * self.zoom))
        height = max(1, int(self.original.height * self.zoom))
        resized = self.original.resize((width, height), Image.Resampling.NEAREST)
        self.tk_image = ImageTk.PhotoImage(resized)
        self.canvas.create_image(self.offset_x, self.offset_y, anchor="nw", image=self.tk_image)

        if self.show_bounds and self.atlas is not None:
            for sprite in self.atlas.sprites:
                x0 = self.offset_x + sprite.x * self.zoom
                y0 = self.offset_y + sprite.y * self.zoom
                x1 = self.offset_x + (sprite.x + sprite.w) * self.zoom
                y1 = self.offset_y + (sprite.y + sprite.h) * self.zoom
                self.canvas.create_rectangle(x0, y0, x1, y1, outline="#2f80ed", width=1)

    def _on_mouse_wheel(self, event: tk.Event) -> None:
        factor = 1.1 if event.delta > 0 else 0.9
        self.zoom_by(factor)

    def _on_drag_start(self, event: tk.Event) -> None:
        self._drag_start = (event.x, event.y)

    def _on_drag_move(self, event: tk.Event) -> None:
        if self._drag_start is None:
            return
        last_x, last_y = self._drag_start
        self.offset_x += event.x - last_x
        self.offset_y += event.y - last_y
        self._drag_start = (event.x, event.y)
        self.redraw()
