from __future__ import annotations

import tkinter as tk
from tkinter import ttk

from hollowatlas.core.types import FileTreeNode, ScanResult


class FileTree(ttk.LabelFrame):
    def __init__(self, master: tk.Misc):
        super().__init__(master, text="Files")
        self.tree = ttk.Treeview(self, show="tree")
        scroll = ttk.Scrollbar(self, orient="vertical", command=self.tree.yview)
        self.tree.configure(yscrollcommand=scroll.set)
        self.tree.grid(row=0, column=0, sticky="nsew")
        scroll.grid(row=0, column=1, sticky="ns")
        self.columnconfigure(0, weight=1)
        self.rowconfigure(0, weight=1)

    def load(self, scan: ScanResult) -> None:
        self.clear()
        root_id = self._insert("", scan.root, open_node=True)
        self.tree.selection_set(root_id)

    def clear(self) -> None:
        for item in self.tree.get_children():
            self.tree.delete(item)

    def _insert(self, parent: str, node: FileTreeNode, open_node: bool = False) -> str:
        label = node.name
        if node.type == "directory":
            label = f"{node.name} ({node.image_count})"
        item_id = self.tree.insert(parent, "end", text=label, open=open_node, values=(node.path,))
        for child in node.children:
            self._insert(item_id, child)
        return item_id
