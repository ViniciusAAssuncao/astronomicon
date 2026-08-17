import tkinter as tk
from tkinter import ttk

from ui.frame_atmosphere import FrameAtmosphere
from ui.frame_barycenter import FrameBarycenter
from ui.frame_planet import FramePlanet
from ui.frame_star import FrameStar
from ui.frame_star_system import FrameStarSystem
from ui.frame_universe_state import FrameUniverseState
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

        self.output_panel = OutputPanel(main_paned)
        main_paned.add(self.output_panel, weight=2)

        self.notebook = ttk.Notebook(top_container)
        self.notebook.grid(row=0, column=0, sticky="nsew", padx=6, pady=6)

        self.frame_system = FrameStarSystem(
            self.notebook,
            output_panel=self.output_panel,
            on_cache_updated=self.on_cache_updated,
        )
        self.frame_star = FrameStar(
            self.notebook,
            output_panel=self.output_panel,
            on_cache_updated=self.on_cache_updated,
        )
        self.frame_planet = FramePlanet(
            self.notebook,
            output_panel=self.output_panel,
            on_cache_updated=self.on_cache_updated,
        )
        self.frame_barycenter = FrameBarycenter(
            self.notebook,
            output_panel=self.output_panel,
            on_cache_updated=self.on_cache_updated,
        )
        self.frame_atmosphere = FrameAtmosphere(
            self.notebook,
            output_panel=self.output_panel,
            on_cache_updated=self.on_cache_updated,
        )
        self.frame_universe_state = FrameUniverseState(
            self.notebook,
            output_panel=self.output_panel,
            on_cache_updated=self.on_cache_updated,
        )

        self.notebook.add(self.frame_system, text="Sistema Estelar")
        self.notebook.add(self.frame_star, text="Estrela")
        self.notebook.add(self.frame_planet, text="Planeta")
        self.notebook.add(self.frame_barycenter, text="Baricentro")
        self.notebook.add(self.frame_atmosphere, text="Atmosfera")
        self.notebook.add(self.frame_universe_state, text="Estado do Universo")

        self.notebook.bind("<<NotebookTabChanged>>", self._on_tab_changed)

    def on_cache_updated(self) -> None:
        self.frame_system.refresh_cache_list()
        self.frame_star.refresh_systems()
        self.frame_star.refresh_cache_list()
        self.frame_planet.refresh_systems()
        self.frame_planet.refresh_cache_list()
        self.frame_barycenter.refresh_systems()
        self.frame_barycenter.refresh_cache_list()
        self.frame_atmosphere.refresh_planets()

    def _on_tab_changed(self, _event: tk.Event) -> None:
        self.on_cache_updated()
