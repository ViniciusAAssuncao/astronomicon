import tkinter as tk
from tkinter import filedialog, messagebox, ttk
from typing import List


class OutputPanel(ttk.Frame):
    def __init__(self, parent: tk.Widget, **kwargs) -> None:
        super().__init__(parent, **kwargs)

        self.session_statements: List[str] = []

        self.columnconfigure(0, weight=1)
        self.rowconfigure(1, weight=1)

        header_frame = ttk.Frame(self)
        header_frame.grid(row=0, column=0, sticky="ew", padx=8, pady=(8, 4))
        header_frame.columnconfigure(0, weight=1)

        title_label = ttk.Label(
            header_frame,
            text="Sessão SQL Acumulada",
            font=("Segoe UI", 10, "bold"),
        )
        title_label.grid(row=0, column=0, sticky="w")

        btn_container = ttk.Frame(header_frame)
        btn_container.grid(row=0, column=1, sticky="e")

        self.copy_btn = ttk.Button(
            btn_container,
            text="Copiar Tudo",
            command=self.copy_to_clipboard,
        )
        self.copy_btn.pack(side=tk.LEFT, padx=(0, 4))

        self.export_btn = ttk.Button(
            btn_container,
            text="Exportar Sessão (.sql)",
            command=self.export_to_file,
        )
        self.export_btn.pack(side=tk.LEFT, padx=(0, 4))

        self.clear_btn = ttk.Button(
            btn_container,
            text="Limpar Sessão",
            command=self.clear_session,
        )
        self.clear_btn.pack(side=tk.LEFT)

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
            text="Sessão vazia. Gere instruções SQL nos formulários acima.",
            font=("Segoe UI", 8),
            foreground="#666666",
        )
        self.status_label.grid(row=2, column=0, sticky="w", padx=8, pady=(0, 4))

    def append_sql(self, sql_text: str) -> None:
        trimmed = sql_text.strip()
        if not trimmed:
            return
        self.session_statements.append(trimmed)
        self._refresh_text_area()

    def set_sql(self, sql_text: str) -> None:
        self.append_sql(sql_text)

    def _refresh_text_area(self) -> None:
        full_text = "\n\n".join(self.session_statements)
        self.text_area.configure(state="normal")
        self.text_area.delete("1.0", tk.END)
        self.text_area.insert("1.0", full_text)
        self.text_area.configure(state="disabled")
        self.text_area.see(tk.END)

        num_statements = len(self.session_statements)
        num_lines = len(full_text.splitlines()) if full_text else 0
        self.status_label.configure(
            text=f"{num_statements} bloco(s) de instrução acumulado(s) na sessão ({num_lines} linhas no total)."
        )

    def clear_session(self) -> None:
        self.session_statements.clear()
        self.text_area.configure(state="normal")
        self.text_area.delete("1.0", tk.END)
        self.text_area.configure(state="disabled")
        self.status_label.configure(text="Sessão limpa.")

    def copy_to_clipboard(self) -> None:
        full_text = "\n\n".join(self.session_statements).strip()
        if not full_text:
            self.status_label.configure(text="Nada para copiar na sessão atual.")
            return

        self.clipboard_clear()
        self.clipboard_append(full_text)
        self.status_label.configure(text="Toda a sessão SQL foi copiada para a área de transferência.")

    def export_to_file(self) -> None:
        full_text = "\n\n".join(self.session_statements).strip()
        if not full_text:
            messagebox.showwarning("Aviso", "Não há comandos SQL na sessão para exportar.")
            return

        filepath = filedialog.asksaveasfilename(
            defaultextension=".sql",
            filetypes=[("Arquivos SQL", "*.sql"), ("Todos os Arquivos", "*.*")],
            title="Salvar Sessão SQL",
        )
        if not filepath:
            return

        try:
            with open(filepath, "w", encoding="utf-8") as f:
                f.write(full_text + "\n")
            messagebox.showinfo("Exportação Concluída", f"Sessão SQL exportada com sucesso em:\n{filepath}")
            self.status_label.configure(text=f"Sessão exportada para {filepath}")
        except Exception as e:
            messagebox.showerror("Erro ao Exportar", f"Falha ao salvar o arquivo:\n{e}")
