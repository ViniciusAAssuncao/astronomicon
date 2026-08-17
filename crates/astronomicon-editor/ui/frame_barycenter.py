import tkinter as tk
from tkinter import messagebox, ttk
from typing import Any, Callable, Dict, Optional

import cache
from curation import curate_barycenter
from models import Barycenter
import sql_builder
from ui.output_panel import OutputPanel
from ui.widgets_common import BarycenterMemberSelector, OrbitalElementsFrame, OrbitalParentSelector
from validation import validate_barycenter


class FrameBarycenter(ttk.Frame):
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
        self.current_barycenter_id: str = sql_builder.generate_uuid()
        self.systems_map: Dict[str, str] = {}
        self.load_cache_map: Dict[str, str] = {}

        self.columnconfigure(0, weight=1)

        cache_load_frame = ttk.LabelFrame(self, text="Carregar / Duplicar do Cache", padding=8)
        cache_load_frame.grid(row=0, column=0, sticky="ew", padx=6, pady=(0, 6))

        ttk.Label(cache_load_frame, text="Baricentro Existente:", width=20, anchor="w").pack(side=tk.LEFT)
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

        info_frame = ttk.LabelFrame(self, text="Identificação do Baricentro", padding=10)
        info_frame.grid(row=2, column=0, sticky="ew", padx=6, pady=4)

        id_row = ttk.Frame(info_frame)
        id_row.pack(fill=tk.X, pady=3)
        ttk.Label(id_row, text="UUID do Baricentro:", width=22, anchor="w").pack(side=tk.LEFT)
        self.id_var = tk.StringVar(value=self.current_barycenter_id)
        self.id_entry = ttk.Entry(id_row, textvariable=self.id_var, width=38)
        self.id_entry.pack(side=tk.LEFT, padx=(0, 6))
        ttk.Button(id_row, text="Novo UUID", command=self._regenerate_uuid).pack(side=tk.LEFT)

        name_row = ttk.Frame(info_frame)
        name_row.pack(fill=tk.X, pady=3)
        ttk.Label(name_row, text="Nome:", width=22, anchor="w").pack(side=tk.LEFT)
        self.name_var = tk.StringVar()
        self.name_entry = ttk.Entry(name_row, textvariable=self.name_var, width=38)
        self.name_entry.pack(side=tk.LEFT)

        members_frame = ttk.LabelFrame(self, text="Membros do Baricentro (Par Binário Interno)", padding=10)
        members_frame.grid(row=3, column=0, sticky="ew", padx=6, pady=4)

        self.primary_selector = BarycenterMemberSelector(members_frame, "Membro Primário:")
        self.primary_selector.pack(fill=tk.X, pady=3)

        self.secondary_selector = BarycenterMemberSelector(members_frame, "Membro Secundário:")
        self.secondary_selector.pack(fill=tk.X, pady=3)

        self.internal_elements_frame = OrbitalElementsFrame(
            self,
            "Elementos Orbitais Internos (Relativos do Par Binário)",
        )
        self.internal_elements_frame.grid(row=4, column=0, sticky="ew", padx=6, pady=4)

        parent_frame = ttk.LabelFrame(self, text="Hierarquia Orbital Externa (Opcional)", padding=10)
        parent_frame.grid(row=5, column=0, sticky="ew", padx=6, pady=4)
        parent_frame.columnconfigure(0, weight=1)

        self.parent_selector = OrbitalParentSelector(
            parent_frame,
            on_parent_type_changed=self._on_parent_type_changed,
            allowed_parent_types=["Estrela", "Planeta", "Baricentro"],
        )
        self.parent_selector.grid(row=0, column=0, sticky="w", pady=(0, 6))

        self.external_elements_frame = OrbitalElementsFrame(
            parent_frame,
            "Elementos Orbitais Externos do Baricentro",
        )
        self.external_elements_frame.grid(row=1, column=0, sticky="ew")
        self.external_elements_frame.set_enabled(False)

        actions_frame = ttk.Frame(self)
        actions_frame.grid(row=6, column=0, sticky="ew", padx=6, pady=10)

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
        self.current_barycenter_id = sql_builder.generate_uuid()
        self.id_var.set(self.current_barycenter_id)

    def refresh_cache_list(self) -> None:
        rows = cache.list_barycenters()
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
        bary_id = self.load_cache_map.get(self.load_cache_var.get())
        if not bary_id:
            messagebox.showwarning("Aviso", "Selecione um baricentro do cache para carregar.")
            return

        entity = cache.get_entity(bary_id)
        if not entity:
            messagebox.showerror("Erro", "Baricentro não encontrado no cache.")
            return

        self._regenerate_uuid()
        self.name_var.set(f"{entity.get('name', '')} (Cópia)")

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
            self.primary_selector.set_system_id(None)
            self.secondary_selector.set_system_id(None)
            self.parent_selector.set_system_id(None)

    def _on_system_selected(self, _event: Optional[tk.Event] = None) -> None:
        sys_id = self.systems_map.get(self.system_var.get())
        self.primary_selector.set_system_id(sys_id)
        self.secondary_selector.set_system_id(sys_id)
        self.parent_selector.set_system_id(sys_id)

    def _on_parent_type_changed(self, parent_type: str) -> None:
        if parent_type == "Fixo (sem pai)":
            self.external_elements_frame.set_enabled(False)
            self.external_elements_frame.clear()
        else:
            self.external_elements_frame.set_enabled(True)

    def build_model(self) -> Barycenter:
        sys_id = self.systems_map.get(self.system_var.get())
        pri_star, pri_planet, pri_bary = self.primary_selector.get_member_references()
        sec_star, sec_planet, sec_bary = self.secondary_selector.get_member_references()
        p_star, p_planet, p_bary = self.parent_selector.get_parent_references()

        int_elems = self.internal_elements_frame.get_elements()
        ext_elems = self.external_elements_frame.get_elements()

        return Barycenter(
            id=self.id_var.get().strip(),
            name=self.name_var.get().strip(),
            star_system_id=sys_id,
            primary_star_id=pri_star,
            primary_planet_id=pri_planet,
            primary_barycenter_id=pri_bary,
            secondary_star_id=sec_star,
            secondary_planet_id=sec_planet,
            secondary_barycenter_id=sec_bary,
            internal_semi_major_axis_m=int_elems.semi_major_axis_m if int_elems else 0.0,
            internal_eccentricity=int_elems.eccentricity if int_elems else 0.0,
            internal_inclination_rad=int_elems.inclination_rad if int_elems else 0.0,
            internal_longitude_ascending_node_rad=int_elems.longitude_ascending_node_rad if int_elems else 0.0,
            internal_argument_periapsis_rad=int_elems.argument_periapsis_rad if int_elems else 0.0,
            internal_mean_anomaly_at_epoch_rad=int_elems.mean_anomaly_at_epoch_rad if int_elems else 0.0,
            parent_star_id=p_star,
            parent_planet_id=p_planet,
            parent_barycenter_id=p_bary,
            external_semi_major_axis_m=ext_elems.semi_major_axis_m if ext_elems else None,
            external_eccentricity=ext_elems.eccentricity if ext_elems else None,
            external_inclination_rad=ext_elems.inclination_rad if ext_elems else None,
            external_longitude_ascending_node_rad=ext_elems.longitude_ascending_node_rad if ext_elems else None,
            external_argument_periapsis_rad=ext_elems.argument_periapsis_rad if ext_elems else None,
            external_mean_anomaly_at_epoch_rad=ext_elems.mean_anomaly_at_epoch_rad if ext_elems else None,
        )

    def generate_sql(self) -> Optional[Barycenter]:
        model = self.build_model()
        errors = validate_barycenter(model)

        pri_id = self.primary_selector.get_entity_id()
        sec_id = self.secondary_selector.get_entity_id()

        if pri_id:
            is_mem, b_id, b_name = cache.is_barycenter_member(pri_id)
            if is_mem and b_id != model.id:
                errors.append(f"Membro primário já pertence ao baricentro '{b_name}' ({b_id})")

        if sec_id:
            is_mem, b_id, b_name = cache.is_barycenter_member(sec_id)
            if is_mem and b_id != model.id:
                errors.append(f"Membro secundário já pertence ao baricentro '{b_name}' ({b_id})")

        if errors:
            messagebox.showerror("Erro de Validação", "\n".join(f"• {e}" for e in errors))
            return None

        warnings = curate_barycenter(model)
        if warnings:
            msg = (
                "Foram detectadas anomalias físicas nos parâmetros:\n\n"
                + "\n".join(f"• {w}" for w in warnings)
                + "\n\nDeseja prosseguir com a geração mesmo assim?"
            )
            if not messagebox.askyesno("Avisos de Curadoria Física", msg, icon="warning"):
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
        messagebox.showinfo("Sucesso", f"Baricentro '{model.name}' registrado no cache!")

    def clear_form(self) -> None:
        self._regenerate_uuid()
        self.name_var.set("")
        self.primary_selector.clear()
        self.secondary_selector.clear()
        self.internal_elements_frame.clear()
        self.parent_selector.clear()
        self.external_elements_frame.clear()
        self.external_elements_frame.set_enabled(False)