import tkinter as tk
from tkinter import messagebox, ttk
from typing import Any, Callable, Dict, Optional

import cache
import sql_builder
import units
from models import STAR_KINDS, Star
from ui.output_panel import OutputPanel
from ui.widgets_common import OrbitalElementsFrame, OrbitalParentSelector, UnitEntry
from validation import validate_star


class FrameStar(ttk.Frame):
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
        self.current_star_id: str = sql_builder.generate_uuid()
        self.systems_map: Dict[str, str] = {}
        self.load_cache_map: Dict[str, str] = {}

        self.columnconfigure(0, weight=1)

        cache_load_frame = ttk.LabelFrame(self, text="Carregar / Duplicar do Cache", padding=8)
        cache_load_frame.grid(row=0, column=0, sticky="ew", padx=6, pady=(0, 6))

        ttk.Label(cache_load_frame, text="Estrela Existente:", width=20, anchor="w").pack(side=tk.LEFT)
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

        sys_frame = ttk.LabelFrame(self, text="Vinculação de Sistema Estelar", padding=10)
        sys_frame.grid(row=1, column=0, sticky="ew", padx=6, pady=4)

        ttk.Label(sys_frame, text="Sistema Estelar:", width=22, anchor="w").pack(side=tk.LEFT)
        self.system_var = tk.StringVar()
        self.system_combo = ttk.Combobox(
            sys_frame,
            textvariable=self.system_var,
            values=[],
            state="readonly",
            width=38,
        )
        self.system_combo.pack(side=tk.LEFT, padx=(0, 6))
        self.system_combo.bind("<<ComboboxSelected>>", self._on_system_selected)

        ttk.Button(sys_frame, text="Atualizar", command=self.refresh_systems).pack(side=tk.LEFT)

        info_frame = ttk.LabelFrame(self, text="Propriedades Físicas e Tipo", padding=10)
        info_frame.grid(row=2, column=0, sticky="ew", padx=6, pady=4)
        info_frame.columnconfigure(0, weight=1)
        info_frame.columnconfigure(1, weight=1)

        id_row = ttk.Frame(info_frame)
        id_row.grid(row=0, column=0, columnspan=2, sticky="ew", pady=3)
        ttk.Label(id_row, text="UUID da Estrela:", width=22, anchor="w").pack(side=tk.LEFT)
        self.id_var = tk.StringVar(value=self.current_star_id)
        self.id_entry = ttk.Entry(id_row, textvariable=self.id_var, width=38)
        self.id_entry.pack(side=tk.LEFT, padx=(0, 6))
        ttk.Button(id_row, text="Novo UUID", command=self._regenerate_uuid).pack(side=tk.LEFT)

        name_row = ttk.Frame(info_frame)
        name_row.grid(row=1, column=0, sticky="ew", pady=3)
        ttk.Label(name_row, text="Nome:", width=22, anchor="w").pack(side=tk.LEFT)
        self.name_var = tk.StringVar()
        self.name_entry = ttk.Entry(name_row, textvariable=self.name_var, width=24)
        self.name_entry.pack(side=tk.LEFT)

        kind_row = ttk.Frame(info_frame)
        kind_row.grid(row=1, column=1, sticky="ew", pady=3)
        ttk.Label(kind_row, text="Classificação (Kind):", width=20, anchor="w").pack(side=tk.LEFT)
        self.kind_var = tk.StringVar(value=STAR_KINDS[0])
        self.kind_combo = ttk.Combobox(
            kind_row,
            textvariable=self.kind_var,
            values=STAR_KINDS,
            state="readonly",
            width=18,
        )
        self.kind_combo.pack(side=tk.LEFT)

        self.entry_mass = UnitEntry(
            info_frame,
            "Massa:",
            [("M☉", units.solar_masses_to_kg), ("kg", lambda x: x), ("M⊕", units.earth_masses_to_kg)],
            default_unit_idx=0,
        )
        self.entry_mass.grid(row=2, column=0, sticky="ew", pady=3)

        self.entry_radius = UnitEntry(
            info_frame,
            "Raio:",
            [("R☉", units.solar_radii_to_meters), ("km", units.km_to_meters), ("m", lambda x: x)],
            default_unit_idx=0,
        )
        self.entry_radius.grid(row=2, column=1, sticky="ew", pady=3)

        self.entry_temp = UnitEntry(
            info_frame,
            "Temperatura Efetiva:",
            [("K", lambda x: x)],
            default_unit_idx=0,
        )
        self.entry_temp.grid(row=3, column=0, sticky="ew", pady=3)

        self.entry_rotation = UnitEntry(
            info_frame,
            "Período de Rotação:",
            [("dias", units.days_to_seconds), ("horas", units.hours_to_seconds), ("s", lambda x: x)],
            default_unit_idx=0,
        )
        self.entry_rotation.grid(row=3, column=1, sticky="ew", pady=3)

        self.entry_axial_tilt = UnitEntry(
            info_frame,
            "Obliquidade Axial:",
            [("graus", units.degrees_to_radians), ("rad", lambda x: x)],
            default_unit_idx=0,
        )
        self.entry_axial_tilt.grid(row=4, column=0, sticky="ew", pady=3)

        self.entry_j2 = UnitEntry(
            info_frame,
            "Achatamento J2:",
            [("-", lambda x: x)],
            default_unit_idx=0,
        )
        self.entry_j2.grid(row=4, column=1, sticky="ew", pady=3)

        parent_frame = ttk.LabelFrame(self, text="Hierarquia Orbital", padding=10)
        parent_frame.grid(row=3, column=0, sticky="ew", padx=6, pady=4)
        parent_frame.columnconfigure(0, weight=1)

        self.parent_selector = OrbitalParentSelector(
            parent_frame,
            on_parent_type_changed=self._on_parent_type_changed,
        )
        self.parent_selector.grid(row=0, column=0, sticky="w", pady=(0, 6))

        self.orbital_elements_frame = OrbitalElementsFrame(parent_frame, "Elementos Orbitais da Estrela")
        self.orbital_elements_frame.grid(row=1, column=0, sticky="ew")
        self.orbital_elements_frame.set_enabled(False)

        actions_frame = ttk.Frame(self)
        actions_frame.grid(row=4, column=0, sticky="ew", padx=6, pady=10)

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

        self.refresh_systems()
        self.refresh_cache_list()

    def _regenerate_uuid(self) -> None:
        self.current_star_id = sql_builder.generate_uuid()
        self.id_var.set(self.current_star_id)

    def refresh_cache_list(self) -> None:
        rows = cache.list_stars()
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
        star_id = self.load_cache_map.get(self.load_cache_var.get())
        if not star_id:
            messagebox.showwarning("Aviso", "Selecione uma estrela do cache para carregar.")
            return

        entity = cache.get_entity(star_id)
        if not entity:
            messagebox.showerror("Erro", "Estrela não encontrada no cache.")
            return

        self._regenerate_uuid()
        self.name_var.set(f"{entity.get('name', '')} (Cópia)")
        kind = entity.get("kind")
        if kind in STAR_KINDS:
            self.kind_var.set(kind)

        sys_id = entity.get("star_system_id")
        if sys_id:
            for display, sid in self.systems_map.items():
                if sid == sys_id:
                    self.system_var.set(display)
                    self._on_system_selected()
                    break

        messagebox.showinfo("Carregado", "Dados carregados com um novo UUID gerado para duplicação.")

    def refresh_systems(self) -> None:
        rows = cache.list_star_systems()
        self.systems_map.clear()
        displays = []
        for r in rows:
            d = f"{r['name']} ({r['id'][:8]}...)"
            self.systems_map[d] = r["id"]
            displays.append(d)
        self.system_combo.configure(values=displays)
        if displays:
            if self.system_var.get() not in displays:
                self.system_var.set(displays[0])
                self._on_system_selected()
        else:
            self.system_var.set("")
            self.parent_selector.set_system_id(None)

    def _on_system_selected(self, _event: Optional[tk.Event] = None) -> None:
        sys_id = self.systems_map.get(self.system_var.get())
        self.parent_selector.set_system_id(sys_id)

    def _on_parent_type_changed(self, parent_type: str) -> None:
        if parent_type == "Fixo (sem pai)":
            self.orbital_elements_frame.set_enabled(False)
            self.orbital_elements_frame.clear()
        else:
            self.orbital_elements_frame.set_enabled(True)

    def build_model(self) -> Star:
        sys_id = self.systems_map.get(self.system_var.get())
        p_star, p_planet, p_bary = self.parent_selector.get_parent_references()
        elems = self.orbital_elements_frame.get_elements()

        return Star(
            id=self.id_var.get().strip(),
            name=self.name_var.get().strip(),
            kind=self.kind_var.get(),
            mass_kg=self.entry_mass.get_si_value() or 0.0,
            star_system_id=sys_id,
            parent_star_id=p_star,
            parent_planet_id=p_planet,
            parent_barycenter_id=p_bary,
            radius_m=self.entry_radius.get_si_value(),
            effective_temperature_k=self.entry_temp.get_si_value(),
            rotation_period_s=self.entry_rotation.get_si_value(),
            axial_tilt_rad=self.entry_axial_tilt.get_si_value(),
            semi_major_axis_m=elems.semi_major_axis_m if elems else None,
            eccentricity=elems.eccentricity if elems else None,
            inclination_rad=elems.inclination_rad if elems else None,
            longitude_ascending_node_rad=elems.longitude_ascending_node_rad if elems else None,
            argument_periapsis_rad=elems.argument_periapsis_rad if elems else None,
            mean_anomaly_at_epoch_rad=elems.mean_anomaly_at_epoch_rad if elems else None,
            oblateness_j2=self.entry_j2.get_si_value(),
        )

    def generate_sql(self) -> Optional[Star]:
        model = self.build_model()
        errors = validate_star(model)
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

        cache.register_entity_from_model(model)
        if self.on_cache_updated:
            self.on_cache_updated()
        self.refresh_cache_list()
        messagebox.showinfo("Sucesso", f"Estrela '{model.name}' registrada no cache!")

    def clear_form(self) -> None:
        self._regenerate_uuid()
        self.name_var.set("")
        self.kind_var.set(STAR_KINDS[0])
        self.entry_mass.clear()
        self.entry_radius.clear()
        self.entry_temp.clear()
        self.entry_rotation.clear()
        self.entry_axial_tilt.clear()
        self.entry_j2.clear()
        self.parent_selector.clear()
        self.orbital_elements_frame.clear()
        self.orbital_elements_frame.set_enabled(False)
