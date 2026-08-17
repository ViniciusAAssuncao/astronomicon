import tkinter as tk
from tkinter import ttk


class OutputPanel(ttk.Frame):
    def __init__(self, parent: tk.Widget, **kwargs) -> None:
        super().__init__(parent, **kwargs)

        self.columnconfigure(0, weight=1)
        self.rowconfigure(1, weight=1)

        header_frame = ttk.Frame(self)
        header_frame.grid(row=0, column=0, sticky="ew", padx=8, pady=(8, 4))
        header_frame.columnconfigure(0, weight=1)

        title_label = ttk.Label(
            header_frame,
            text="SQL Gerado",
            font=("Segoe UI", 10, "bold"),
        )
        title_label.grid(row=0, column=0, sticky="w")

        self.copy_btn = ttk.Button(
            header_frame,
            text="Copiar SQL",
            command=self.copy_to_clipboard,
        )
        self.copy_btn.grid(row=0, column=1, sticky="e", padx=(4, 0))

        self.clear_btn = ttk.Button(
            header_frame,
            text="Limpar",
            command=self.clear_sql,
        )
        self.clear_btn.grid(row=0, column=2, sticky="e", padx=(4, 0))

        text_frame = ttk.Frame(self)
        text_frame.grid(row=1, column=0, sticky="nsew", padx=8, pady=(0, 8))
        text_frame.columnconfigure(0, weight=1)
        text_frame.rowconfigure(0, weight=1)

        self.text_area = tk.Text(
            text_frame,
            wrap="none",
            font=("Consolas", 9),
            bg="#1E1E1E",
            fg="#D4D4D4",
            insertbackground="#FFFFFF",
            selectbackground="#264F78",
            selectforeground="#FFFFFF",
            relief="flat",
            state="disabled",
            height=12,
        )
        self.text_area.grid(row=0, column=0, sticky="nsew")

        v_scroll = ttk.Scrollbar(
            text_frame,
            orient="vertical",
            command=self.text_area.yview,
        )
        v_scroll.grid(row=0, column=1, sticky="ns")

        h_scroll = ttk.Scrollbar(
            text_frame,
            orient="horizontal",
            command=self.text_area.xview,
        )
        h_scroll.grid(row=1, column=0, sticky="ew")

        self.text_area.configure(
            xscrollcommand=h_scroll.set,
            yscrollcommand=v_scroll.set,
        )

        self.status_label = ttk.Label(
            self,
            text="",
            font=("Segoe UI", 8),
            foreground="#666666",
        )
        self.status_label.grid(row=2, column=0, sticky="w", padx=8, pady=(0, 4))

    def set_sql(self, sql_text: str) -> None:
        self.text_area.configure(state="normal")
        self.text_area.delete("1.0", tk.END)
        self.text_area.insert("1.0", sql_text.strip())
        self.text_area.configure(state="disabled")
        self.status_label.configure(text=f"{len(sql_text.strip().splitlines())} linha(s) de SQL")

    def clear_sql(self) -> None:
        self.text_area.configure(state="normal")
        self.text_area.delete("1.0", tk.END)
        self.text_area.configure(state="disabled")
        self.status_label.configure(text="")

    def copy_to_clipboard(self) -> None:
        self.text_area.configure(state="normal")
        content = self.text_area.get("1.0", tk.END).strip()
        self.text_area.configure(state="disabled")

        if not content:
            self.status_label.configure(text="Nada para copiar")
            return

        self.clipboard_clear()
        self.clipboard_append(content)
        self.status_label.configure(text="SQL copiado para a área de transferência com sucesso")