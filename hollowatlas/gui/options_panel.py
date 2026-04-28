from __future__ import annotations

import tkinter as tk
from tkinter import ttk

from hollowatlas.core.types import OutputFormat, PackConfig, SplitMode


class OptionsPanel(ttk.LabelFrame):
    def __init__(self, master: tk.Misc):
        super().__init__(master, text="Options")

        self.max_size = tk.StringVar(value="2048")
        self.padding = tk.StringVar(value="2")
        self.extrude = tk.StringVar(value="1")
        self.trim = tk.BooleanVar(value=True)
        self.allow_rotation = tk.BooleanVar(value=False)
        self.power_of_two = tk.BooleanVar(value=True)
        self.square = tk.BooleanVar(value=True)
        self.split_mode = tk.StringVar(value=SplitMode.ALL_IN_ONE.value)
        self.output_format = tk.StringVar(value=OutputFormat.GODOT_TPSHEET.value)
        self.debug_json = tk.BooleanVar(value=False)
        self.show_bounds = tk.BooleanVar(value=True)

        row = 0
        self._combo(row, "Max Size", self.max_size, ["512", "1024", "2048", "4096", "8192"])
        row += 1
        self._combo(row, "Padding", self.padding, ["0", "1", "2", "4", "8"])
        row += 1
        self._combo(row, "Extrude", self.extrude, ["0", "1", "2", "4"])
        row += 1
        self._check(row, "Trim Transparent", self.trim)
        row += 1
        self._check(row, "Allow Rotation", self.allow_rotation)
        row += 1
        self._check(row, "Power of Two", self.power_of_two)
        row += 1
        self._check(row, "Square Atlas", self.square)
        row += 1
        self._combo(row, "Split Mode", self.split_mode, [mode.value for mode in SplitMode])
        row += 1
        self._combo(row, "Output Format", self.output_format, [fmt.value for fmt in OutputFormat])
        row += 1
        self._check(row, "Debug JSON", self.debug_json)
        row += 1
        self._check(row, "Show Bounds", self.show_bounds)

        self.columnconfigure(1, weight=1)

    def _combo(self, row: int, label: str, variable: tk.StringVar, values: list[str]) -> None:
        ttk.Label(self, text=label).grid(row=row, column=0, sticky="w", padx=8, pady=5)
        box = ttk.Combobox(self, textvariable=variable, values=values, state="readonly", width=18)
        box.grid(row=row, column=1, sticky="ew", padx=8, pady=5)

    def _check(self, row: int, label: str, variable: tk.BooleanVar) -> None:
        ttk.Checkbutton(self, text=label, variable=variable).grid(
            row=row,
            column=0,
            columnspan=2,
            sticky="w",
            padx=8,
            pady=5,
        )

    def get_config(self) -> PackConfig:
        return PackConfig(
            max_size=int(self.max_size.get()),
            padding=int(self.padding.get()),
            extrude=int(self.extrude.get()),
            trim=self.trim.get(),
            allow_rotation=self.allow_rotation.get(),
            power_of_two=self.power_of_two.get(),
            square=self.square.get(),
            split_mode=SplitMode(self.split_mode.get()),
            output_format=OutputFormat(self.output_format.get()),
            debug_json=self.debug_json.get() or self.output_format.get() == OutputFormat.JSON_DEBUG.value,
        )
