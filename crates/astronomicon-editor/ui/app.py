import tkinter as tk
from tkinter import ttk
from ui.output_panel import OutputPanel


class App(tk.Tk):
    def __init__(self) -> None:
        super().__init__()

        self.title("Astronomicon - Editor de Entidades")
        self.geometry("1100x750")
        self.minsize(850, 600)

        self.style = ttk.Style(self)
        try:
            self.style.theme_use("clam")
        except tk.TclError:
            pass

        self.style.configure(".", font=("Segoe UI", 9))
        self.style.configure("TNotebook.Tab", padding=(12, 6), font=("Segoe UI", 9, "bold"))
        self.style.configure("Header.TLabel", font=("Segoe UI", 12, "bold"))

        self.columnconfigure(0, weight=1)
        self.rowconfigure(0, weight=1)

        main_paned = ttk.PanedWindow(self, orient=tk.VERTICAL)
        main_paned.grid(row=0, column=0, sticky="nsew")

        top_container = ttk.Frame(main_paned)
        top_container.columnconfigure(0, weight=1)
        top_container.rowconfigure(0, weight=1)
        main_paned.add(top_container, weight=3)

        self.notebook = ttk.Notebook(top_container)
        self.notebook.grid(row=0, column=0, sticky="nsew", padx=6, pady=6)

        self.tab_system = ttk.Frame(self.notebook)
        self.tab_star = ttk.Frame(self.notebook)
        self.tab_planet = ttk.Frame(self.notebook)
        self.tab_barycenter = ttk.Frame(self.notebook)
        self.tab_atmosphere = ttk.Frame(self.notebook)
        self.tab_universe_state = ttk.Frame(self.notebook)

        self.notebook.add(self.tab_system, text="Sistema Estelar")
        self.notebook.add(self.tab_star, text="Estrela")
        self.notebook.add(self.tab_planet, text="Planeta")
        self.notebook.add(self.tab_barycenter, text="Baricentro")
        self.notebook.add(self.tab_atmosphere, text="Atmosfera")
        self.notebook.add(self.tab_universe_state, text="Estado do Universo")

        self.init_placeholder_tabs()

        self.output_panel = OutputPanel(main_paned)
        main_paned.add(self.output_panel, weight=2)

    def init_placeholder_tabs(self) -> None:
        tabs_config = [
            (self.tab_system, "Formulário de Sistema Estelar"),
            (self.tab_star, "Formulário de Estrela"),
            (self.tab_planet, "Formulário de Planeta"),
            (self.tab_barycenter, "Formulário de Baricentro"),
            (self.tab_atmosphere, "Formulário de Atmosfera"),
            (self.tab_universe_state, "Formulário de Estado do Universo"),
        ]

        for tab_frame, label_text in tabs_config:
            tab_frame.columnconfigure(0, weight=1)
            tab_frame.rowconfigure(0, weight=1)
            lbl = ttk.Label(
                tab_frame,
                text=label_text,
                style="Header.TLabel",
                anchor="center",
            )
            lbl.grid(row=0, column=0, sticky="nsew", padx=20, pady=20)