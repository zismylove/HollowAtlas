from __future__ import annotations

import tkinter as tk
from tkinter import ttk


class LogPanel(ttk.Frame):
    def __init__(self, master: tk.Misc):
        super().__init__(master)
        self.text = tk.Text(self, height=8, wrap="word", state="disabled")
        scroll = ttk.Scrollbar(self, orient="vertical", command=self.text.yview)
        self.text.configure(yscrollcommand=scroll.set)
        self.text.grid(row=0, column=0, sticky="nsew")
        scroll.grid(row=0, column=1, sticky="ns")
        self.columnconfigure(0, weight=1)
        self.rowconfigure(0, weight=1)

        self.text.tag_configure("info", foreground="#1f4b73")
        self.text.tag_configure("warning", foreground="#9a6700")
        self.text.tag_configure("error", foreground="#b42318")
        self.text.tag_configure("success", foreground="#067647")

    def add(self, level: str, message: str) -> None:
        self.text.configure(state="normal")
        self.text.insert("end", f"[{level}] {message}\n", level)
        self.text.configure(state="disabled")
        self.text.see("end")

    def clear(self) -> None:
        self.text.configure(state="normal")
        self.text.delete("1.0", "end")
        self.text.configure(state="disabled")
