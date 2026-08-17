import tkinter as tk
from tkinter import messagebox, ttk
from typing import Any, Callable, Dict, Optional

import cache
import sql_builder
import units
from models import StarSystem
from ui.output_panel import OutputPanel
from ui.widgets_common import UnitEntry
from validation import validate_star_system


class FrameStarSystem(ttk.Frame):
    def __init__(
        self,
        parent: tk.Widget,
        output_panel: OutputPanel,
        on_cache_updated: Optional[Callable[[], None]] = None,
        **kwargs: Any,
    ) -> None:
        super().__init__(parent, padding=12, **kwargs)

        self.output_panel = output_panel
        self.on_cache_updated = on_cache_updated
        self.current_system_id: str = sql_builder.generate_uuid()
        self.load_cache_map: Dict[str, str] = {}

        self.columnconfigure(0, weight=1)

        cache_load_frame = ttk.LabelFrame(self, text="Carregar / Duplicar do Cache", padding=8)
        cache_load_frame.grid(row=0, column=0, sticky="ew", padx=6, pady=(0, 6))

        ttk.Label(cache_load_frame, text="Sistema Existente:", width=20, anchor="w").pack(side=tk.LEFT)
        self.load_cache_var = tk.StringVar()
        self.load_cache_combo = ttk.Combobox(
            cache_load_frame,
            textvariable=self.load_cache_var,
            values=[],
            state="readonly",
            width=36,
        )
        self.load_cache_combo.pack(side=tk.LEFT, padx=(0, 6))

        ttk.Button(cache_load_frame, text="Carregar Dados", command=self.load_from_cache).pack(side=tk.LEFT, padx=(0, 4))
        ttk.Button(cache_load_frame, text="Atualizar", command=self.refresh_cache_list).pack(side=tk.LEFT)

        info_frame = ttk.LabelFrame(self, text="Identificação e Parâmetros", padding=10)
        info_frame.grid(row=1, column=0, sticky="ew", padx=6, pady=4)
        info_frame.columnconfigure(0, weight=1)

        id_row = ttk.Frame(info_frame)
        id_row.grid(row=0, column=0, sticky="ew", pady=3)
        ttk.Label(id_row, text="UUID do Sistema:", width=22, anchor="w").pack(side=tk.LEFT)
        self.id_var = tk.StringVar(value=self.current_system_id)
        self.id_entry = ttk.Entry(id_row, textvariable=self.id_var, width=38)
        self.id_entry.pack(side=tk.LEFT, padx=(0, 6))
        ttk.Button(id_row, text="Novo UUID", command=self._regenerate_uuid).pack(side=tk.LEFT)

        name_row = ttk.Frame(info_frame)
        name_row.grid(row=1, column=0, sticky="ew", pady=3)
        ttk.Label(name_row, text="Nome do Sistema:", width=22, anchor="w").pack(side=tk.LEFT)
        self.name_var = tk.StringVar()
        self.name_entry = ttk.Entry(name_row, textvariable=self.name_var, width=38)
        self.name_entry.pack(side=tk.LEFT)

        coords_frame = ttk.LabelFrame(self, text="Coordenadas Celestiais e Distância", padding=10)
        coords_frame.grid(row=2, column=0, sticky="ew", padx=6, pady=4)
        coords_frame.columnconfigure(0, weight=1)

        self.entry_ra = UnitEntry(
            coords_frame,
            "Ascensão Reta (α):",
            [("graus", units.degrees_to_radians), ("rad", lambda x: x)],
            default_unit_idx=0,
        )
        self.entry_ra.grid(row=0, column=0, sticky="w", pady=3)

        self.entry_dec = UnitEntry(
            coords_frame,
            "Declinação (δ):",
            [("graus", units.degrees_to_radians), ("rad", lambda x: x)],
            default_unit_idx=0,
        )
        self.entry_dec.grid(row=1, column=0, sticky="w", pady=3)

        self.entry_dist = UnitEntry(
            coords_frame,
            "Distância de Sol:",
            [
                ("ly", units.ly_to_meters),
                ("pc", units.parsec_to_meters),
                ("AU", units.au_to_meters),
                ("m", lambda x: x),
            ],
            default_unit_idx=0,
        )
        self.entry_dist.grid(row=2, column=0, sticky="w", pady=3)

        actions_frame = ttk.Frame(self)
        actions_frame.grid(row=3, column=0, sticky="ew", padx=6, pady=12)

        ttk.Button(
            actions_frame,
            text="Gerar SQL",
            command=self.generate_sql,
        ).pack(side=tk.LEFT, padx=(0, 6))

        ttk.Button(
            actions_frame,
            text="Registrar no Cache",
            command=self.register_in_cache,
        ).pack(side=tk.LEFT, padx=(0, 6))

        ttk.Button(
            actions_frame,
            text="Limpar Formulário",
            command=self.clear_form,
        ).pack(side=tk.LEFT)

        self.refresh_cache_list()

    def _regenerate_uuid(self) -> None:
        self.current_system_id = sql_builder.generate_uuid()
        self.id_var.set(self.current_system_id)

    def refresh_cache_list(self) -> None:
        rows = cache.list_star_systems()
        self.load_cache_map.clear()
        displays = []
        for r in rows:
            d = f"{r['name']} ({r['id'][:8]}...)"
            self.load_cache_map[d] = r["id"]
            displays.append(d)
        self.load_cache_combo.configure(values=displays)
        if displays:
            if self.load_cache_var.get() not in displays:
                self.load_cache_var.set(displays[0])
        else:
            self.load_cache_var.set("")

    def load_from_cache(self) -> None:
        sys_id = self.load_cache_map.get(self.load_cache_var.get())
        if not sys_id:
            messagebox.showwarning("Aviso", "Selecione um sistema do cache para carregar.")
            return

        entity = cache.get_entity(sys_id)
        if not entity:
            messagebox.showerror("Erro", "Sistema não encontrado no cache.")
            return

        self._regenerate_uuid()
        self.name_var.set(f"{entity.get('name', '')} (Cópia)")
        messagebox.showinfo("Carregado", "Dados carregados com um novo UUID gerado para duplicação.")

    def build_model(self) -> StarSystem:
        return StarSystem(
            id=self.id_var.get().strip(),
            name=self.name_var.get().strip(),
            right_ascension_rad=self.entry_ra.get_si_value(),
            declination_rad=self.entry_dec.get_si_value(),
            distance_from_sol_m=self.entry_dist.get_si_value(),
        )

    def generate_sql(self) -> Optional[StarSystem]:
        model = self.build_model()
        errors = validate_star_system(model)
        if errors:
            messagebox.showerror("Erro de Validação", "\n".join(f"• {e}" for e in errors))
            return None

        sql = sql_builder.build_insert_sql(model, atomic=True)
        self.output_panel.append_sql(sql)
        return model

    def register_in_cache(self) -> None:
        model = self.generate_sql()
        if not model:
            return

        cache.register_star_system(model.id, model.name)
        if self.on_cache_updated:
            self.on_cache_updated()
        self.refresh_cache_list()
        messagebox.showinfo("Sucesso", f"Sistema '{model.name}' registrado no cache!")

    def clear_form(self) -> None:
        self._regenerate_uuid()
        self.name_var.set("")
        self.entry_ra.clear()
        self.entry_dec.clear()
        self.entry_dist.clear()
